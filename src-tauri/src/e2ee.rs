//! Criptografia de ponta a ponta na leitura e no envio (US13): decifra e verifica ao abrir, guarda as
//! chaves que chegam por Autocrypt ou anexo, monta a mensagem cifrada ou assinada e confere quais
//! destinatários têm chave. Logs: só resultados, nunca endereços, chaves ou conteúdo.

use mail_parser::{MessageParser, MimeHeaders};
use pgp::composed::{SignedPublicKey, SignedSecretKey};
use serde::Serialize;

use crate::error::AppError;
use crate::openpgp::{self, ParsedKey, Verification};
use crate::pgp_mime::{self, Found};
use crate::pgp_store;
use crate::session::AppState;

/// O que a leitura mostra sobre a segurança de uma mensagem (CA7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Security {
    pub technology: &'static str,
    pub encrypted: bool,
    /// Cifrada e aberta com uma chave minha.
    pub decrypted: bool,
    pub post_quantum: bool,
    pub signature: Verification,
    /// Aceitação da chave que assinou ("verified", "unverified", "undecided"…), quando conhecida.
    pub signer_acceptance: Option<String>,
}

/// A mensagem pronta para `build_view`, com o que a criptografia acrescentou.
pub struct Opened {
    pub raw: Vec<u8>,
    pub security: Option<Security>,
    /// Assunto verdadeiro de uma mensagem com o assunto cifrado.
    pub subject: Option<String>,
}

fn sender_of(raw: &[u8]) -> Option<String> {
    let message = MessageParser::default().parse_headers(raw)?;
    message.from()?.first()?.address().map(|a| a.trim().to_lowercase())
}

/// Chaves que chegam numa mensagem: o cabeçalho Autocrypt do remetente e anexos `application/pgp-keys`.
fn incoming_keys(raw: &[u8], sender: &str) -> Vec<(SignedPublicKey, &'static str)> {
    let Some(message) = MessageParser::default().parse(raw) else { return Vec::new() };
    let mut out = Vec::new();
    if let Some(value) = message.header_raw("Autocrypt") {
        if let Some(bytes) = pgp_mime::autocrypt_key(value, sender) {
            if let Ok(keys) = openpgp::parse_keys(&bytes) {
                out.extend(keys.iter().map(|k| (k.public(), "autocrypt")));
            }
        }
    }
    for part in message.attachments() {
        let is_key = part.content_type().is_some_and(|ct| ct.ctype().eq_ignore_ascii_case("application") && ct.subtype().is_some_and(|s| s.eq_ignore_ascii_case("pgp-keys")));
        if is_key {
            if let Ok(keys) = openpgp::parse_keys(part.contents()) {
                // Só a pública: uma chave secreta num anexo nunca entra no chaveiro sozinha.
                out.extend(keys.iter().map(|k| (k.public(), "attachment")));
            }
        }
    }
    // Só chaves do próprio remetente.
    out.retain(|(k, _)| openpgp::info(k).emails.iter().any(|e| e == sender));
    out
}

/// Abre uma mensagem para a leitura: guarda as chaves recebidas, decifra e verifica.
pub async fn open(state: &AppState, raw: Vec<u8>) -> Opened {
    let Some(sender) = sender_of(&raw) else { return Opened { raw, security: None, subject: None } };
    let found = pgp_mime::find(&raw);
    let encrypted = matches!(found, Found::Encrypted(_));
    let incoming = incoming_keys(&raw, &sender);
    let known = sender.clone();
    let loaded = state
        .db
        .call(move |c| {
            for (key, origin) in &incoming {
                let info = openpgp::info(key);
                // Uma chave nova de um contato entra como "ainda não aceita"; a conhecida só atualiza.
                if let Ok(bytes) = openpgp::public_bytes(key) {
                    pgp_store::upsert_key(c, &info, &bytes, None, origin)?;
                }
            }
            let signers = pgp_store::keys_for(c, &known)?;
            let secrets = if encrypted { pgp_store::all_secret_keys(c)? } else { Vec::new() };
            Ok((signers, secrets))
        })
        .await;
    if matches!(found, Found::None) {
        return Opened { raw, security: None, subject: None };
    }
    let Ok((signer_bytes, secret_bytes)) = loaded else { return Opened { raw, security: None, subject: None } };
    let result = tokio::task::spawn_blocking(move || {
        let signers: Vec<SignedPublicKey> = signer_bytes.iter().filter_map(|b| openpgp::public_from_bytes(b).ok()).collect();
        match found {
            Found::Encrypted(cipher) => {
                let secrets: Vec<SignedSecretKey> = secret_bytes.iter().filter_map(|b| openpgp::secret_from_bytes(b).ok()).collect();
                match openpgp::decrypt(&cipher, &secrets, &signers) {
                    Ok(out) => {
                        let subject = pgp_mime::protected_subject(&out.data);
                        let shown = pgp_mime::rebuild(&raw, &out.data).unwrap_or(raw);
                        let security = Security {
                            technology: "openpgp",
                            encrypted: true,
                            decrypted: true,
                            post_quantum: out.post_quantum,
                            signature: out.verification,
                            signer_acceptance: None,
                        };
                        Opened { raw: shown, security: Some(security), subject }
                    }
                    Err(_) => Opened {
                        raw,
                        security: Some(Security {
                            technology: "openpgp",
                            encrypted: true,
                            decrypted: false,
                            post_quantum: false,
                            signature: Verification::Unsigned,
                            signer_acceptance: None,
                        }),
                        subject: None,
                    },
                }
            }
            Found::Signed { content, signature } => {
                let verification = openpgp::verify_detached(&content, &signature, &signers);
                let security = Security { technology: "openpgp", encrypted: false, decrypted: false, post_quantum: false, signature: verification, signer_acceptance: None };
                Opened { raw, security: Some(security), subject: None }
            }
            Found::None => Opened { raw, security: None, subject: None },
        }
    })
    .await;
    let Ok(mut opened) = result else { return Opened { raw: Vec::new(), security: None, subject: None } };
    // Aceitação da chave que assinou, para o selo (verde só com a chave conferida).
    if let Some(Security { signature: Verification::Good { fingerprint }, .. }) = &opened.security {
        let fp = fingerprint.clone();
        let acceptance = state.db.call(move |c| Ok(pgp_store::get_key(c, &fp)?.map(|k| if k.mine { "verified".to_owned() } else { k.acceptance }))).await.ok().flatten();
        if let Some(sec) = opened.security.as_mut() {
            sec.signer_acceptance = acceptance;
        }
    }
    if let Some(sec) = &opened.security {
        tracing::info!(encrypted = sec.encrypted, decrypted = sec.decrypted, signed = !matches!(sec.signature, Verification::Unsigned), "mensagem com OpenPGP aberta");
    }
    opened
}

// ---------------------------------------------------------------- envio (CA6)

/// Estado da chave de um destinatário no composer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipientKey {
    pub address: String,
    pub has_key: bool,
    pub post_quantum: bool,
}

pub fn recipient_status(c: &rusqlite::Connection, addresses: &[String]) -> Result<Vec<RecipientKey>, AppError> {
    addresses
        .iter()
        .map(|a| {
            let found = pgp_store::usable_key_for(c, a)?;
            let post_quantum = match &found {
                Some((fp, _)) => pgp_store::get_key(c, fp)?.is_some_and(|k| k.post_quantum),
                None => false,
            };
            Ok(RecipientKey { address: a.clone(), has_key: found.is_some(), post_quantum })
        })
        .collect()
}

/// O que o envio precisa para cifrar ou assinar: a minha chave e as dos destinatários.
pub struct SendKeys {
    pub mine: SignedSecretKey,
    pub recipients: Vec<SignedPublicKey>,
}

/// Carrega as chaves do envio. Cifrar com algum destinatário sem chave: `MissingRecipientKey`.
pub fn send_keys(c: &rusqlite::Connection, account_id: &str, recipients: &[String], encrypt: bool) -> Result<SendKeys, AppError> {
    let settings = pgp_store::crypto_settings(c, account_id)?;
    let fp = settings.pgp_key.ok_or(AppError::NoOwnKey)?;
    let secret = pgp_store::secret_key(c, &fp)?.ok_or(AppError::NoOwnKey)?;
    let mine = openpgp::secret_from_bytes(&secret)?;
    let mut keys = Vec::new();
    if encrypt {
        for r in recipients {
            let (_, bytes) = pgp_store::usable_key_for(c, r)?.ok_or(AppError::MissingRecipientKey)?;
            keys.push(openpgp::public_from_bytes(&bytes)?);
        }
        // Também para mim, para eu ler a cópia em Enviados.
        keys.push(mine.to_public_key());
    }
    Ok(SendKeys { mine, recipients: keys })
}

/// Importa chaves de um arquivo ou texto. Chave secreta protegida sem a senha: `WrongPassword`.
pub fn import(c: &mut rusqlite::Connection, data: &[u8], passphrase: Option<&str>) -> Result<Vec<pgp_store::PgpKeyRow>, AppError> {
    let mut out = Vec::new();
    for key in openpgp::parse_keys(data)? {
        match key {
            ParsedKey::Public(k) => {
                let info = openpgp::info(&k);
                out.push(pgp_store::upsert_key(c, &info, &openpgp::public_bytes(&k)?, None, "imported")?);
            }
            ParsedKey::Secret(k) => {
                let open = if openpgp::is_locked(&k) { openpgp::unlock(*k, passphrase.ok_or(AppError::WrongPassword)?)? } else { *k };
                let public = open.to_public_key();
                let info = openpgp::info(&public);
                out.push(pgp_store::upsert_key(c, &info, &openpgp::public_bytes(&public)?, Some(&openpgp::secret_bytes(&open)?), "imported")?);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openpgp::KeyKind;
    use crate::store::tests::{account, test_db};

    #[test]
    fn send_keys_require_own_key_and_every_recipient() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@x.com");
            assert!(matches!(send_keys(c, &a.id, &[], false), Err(AppError::NoOwnKey)));
            let mine = openpgp::generate(KeyKind::Curve25519, "Eu", "eu@x.com").unwrap();
            let row = import(c, openpgp::armored_secret_backup(&mine, "frase longa do backup").unwrap().as_bytes(), Some("frase longa do backup")).unwrap();
            assert!(row[0].mine);
            let s = pgp_store::CryptoSettings { pgp_key: Some(row[0].fingerprint.clone()), ..Default::default() };
            pgp_store::set_crypto_settings(c, &a.id, &s).unwrap();
            assert!(send_keys(c, &a.id, &["ana@x.com".into()], false).is_ok(), "só assinar não precisa da chave dela");
            assert!(matches!(send_keys(c, &a.id, &["ana@x.com".into()], true), Err(AppError::MissingRecipientKey)));

            let ana = openpgp::generate(KeyKind::Curve25519, "Ana", "ana@x.com").unwrap().to_public_key();
            let imported = import(c, openpgp::armored_public(&ana).unwrap().as_bytes(), None).unwrap();
            assert!(!recipient_status(c, &["ana@x.com".into()]).unwrap()[0].has_key, "importada, mas ainda não aceita");
            pgp_store::set_acceptance(c, &imported[0].fingerprint, "verified").unwrap();
            let keys = send_keys(c, &a.id, &["ana@x.com".into()], true).unwrap();
            assert_eq!(keys.recipients.len(), 2, "ela e eu");
            assert!(matches!(import(c, openpgp::armored_secret_backup(&mine, "frase longa do backup").unwrap().as_bytes(), None), Err(AppError::WrongPassword)));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn autocrypt_keys_must_belong_to_the_sender() {
        let ana = openpgp::generate(KeyKind::Curve25519, "Ana", "ana@x.com").unwrap().to_public_key();
        let value = pgp_mime::autocrypt_value("ana@x.com", &openpgp::public_bytes(&ana).unwrap());
        let raw = format!("From: Ana <ana@x.com>\r\nAutocrypt: {value}\r\nSubject: oi\r\n\r\nOi");
        assert_eq!(incoming_keys(raw.as_bytes(), "ana@x.com").len(), 1);
        let forged = raw.replace("From: Ana <ana@x.com>", "From: Golpe <golpe@x.com>");
        assert!(incoming_keys(forged.as_bytes(), "golpe@x.com").is_empty(), "o addr do Autocrypt tem de ser o remetente");
    }
}
