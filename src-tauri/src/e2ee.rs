//! Criptografia de ponta a ponta na leitura e no envio (US13, US14): decifra e verifica ao abrir, guarda
//! as chaves e os certificados que chegam nas mensagens, escolhe as chaves e os certificados do envio e
//! confere quais destinatários têm um ou outro. Logs: só resultados, nunca endereços, chaves ou conteúdo.

use mail_parser::{MessageParser, MimeHeaders};
use pgp::composed::{SignedPublicKey, SignedSecretKey};
use serde::Serialize;

use crate::error::AppError;
use crate::openpgp::{self, ParsedKey, Verification};
use crate::pgp_mime::{self, Found};
use crate::pgp_store;
use crate::session::AppState;
use crate::smime::{self, Trust};
use crate::smime_mime;
use crate::smime_store;
use openssl::x509::X509;

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
    /// S/MIME (US14): o certificado de quem assinou e a cadeia.
    pub certificate: Option<CertCheck>,
}

/// O certificado S/MIME de quem assinou, para o selo (CA3 da US14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertCheck {
    pub subject: String,
    pub issuer: String,
    pub trust: Trust,
    /// O certificado é do endereço que aparece no De.
    pub email_matches: bool,
    pub not_after: i64,
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
    let smime_found = smime_mime::find(&raw);
    if !matches!(smime_found, smime_mime::Found::None) {
        return open_smime(state, raw, sender, smime_found).await;
    }
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
                            certificate: None,
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
                            certificate: None,
                        }),
                        subject: None,
                    },
                }
            }
            Found::Signed { content, signature } => {
                let verification = openpgp::verify_detached(&content, &signature, &signers);
                let security = Security { technology: "openpgp", encrypted: false, decrypted: false, post_quantum: false, signature: verification, signer_acceptance: None, certificate: None };
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

// ---------------------------------------------------------------- S/MIME na leitura (US14, CA3)

/// Abre uma mensagem S/MIME: decifra, verifica a assinatura (destacada ou opaca, também dentro da
/// parte cifrada) e guarda o certificado de quem assinou, se for do remetente.
async fn open_smime(state: &AppState, raw: Vec<u8>, sender: String, found: smime_mime::Found) -> Opened {
    let enveloped = matches!(found, smime_mime::Found::Enveloped(_));
    let who = sender.clone();
    let loaded = state
        .db
        .call(move |c| {
            let known = smime_store::certs_for(c, &who)?;
            let mine = if enveloped { smime_store::my_certs(c)? } else { Vec::new() };
            Ok((known, mine))
        })
        .await;
    let Ok((known, mine)) = loaded else { return Opened { raw, security: None, subject: None } };
    let roots = smime::system_roots();
    let result = tokio::task::spawn_blocking(move || read_smime(raw, &sender, found, &known, &mine, roots)).await;
    let Ok((opened, collected)) = result else { return Opened { raw: Vec::new(), security: None, subject: None } };
    if let Some((cert, chain)) = collected {
        // Certificado novo de um contato: aba Pessoas (CA2). O conhecido só ganha a cadeia.
        let _ = state.db.call(move |c| smime_store::upsert_cert(c, &cert, &chain, None, "message").map(|_| ())).await;
    }
    if let Some(sec) = &opened.security {
        tracing::info!(encrypted = sec.encrypted, decrypted = sec.decrypted, signed = !matches!(sec.signature, Verification::Unsigned), "mensagem com S/MIME aberta");
    }
    opened
}

type Collected = Option<(X509, Vec<X509>)>;

fn read_smime(raw: Vec<u8>, sender: &str, found: smime_mime::Found, known: &[X509], mine: &[smime_store::Stored], roots: &[X509]) -> (Opened, Collected) {
    use smime_mime::Found;
    let mut security = Security {
        technology: "smime",
        encrypted: false,
        decrypted: false,
        post_quantum: false,
        signature: Verification::Unsigned,
        signer_acceptance: None,
        certificate: None,
    };
    let mut shown: Option<Vec<u8>> = None;
    let mut subject = None;
    let mut current = found;
    if let Found::Enveloped(der) = &current {
        security.encrypted = true;
        let keys: Vec<_> = mine.iter().filter_map(|s| Some((s.cert.clone(), smime::private_key_from_der(s.private_key.as_ref()?).ok()?))).collect();
        match smime::decrypt(der, &keys) {
            Ok(inner) => {
                security.decrypted = true;
                subject = pgp_mime::protected_subject(&inner);
                current = smime_mime::find(&inner);
                shown = Some(inner);
            }
            Err(_) => return (Opened { raw, security: Some(security), subject: None }, None),
        }
    }
    let checked = match &current {
        Found::Signed { content, part, signature } => {
            subject = subject.or_else(|| pgp_mime::protected_subject(part));
            let checked = smime::verify(signature, Some(content), known);
            shown = Some(part.clone());
            Some(checked)
        }
        Found::Opaque(der) => Some(smime::verify(der, None, known)),
        _ => None,
    };
    let mut collected = None;
    match checked {
        Some(smime::Checked::Good { content, signer, embedded }) => {
            if matches!(current, Found::Opaque(_)) {
                subject = subject.or_else(|| pgp_mime::protected_subject(&content));
                shown = Some(content);
            }
            let info = smime::info(&signer);
            let chain: Vec<X509> = embedded.into_iter().filter(|c| smime::fingerprint(c) != info.fingerprint).collect();
            let email_matches = info.emails.iter().any(|e| e == sender);
            security.certificate = Some(CertCheck {
                subject: info.subject.clone(),
                issuer: info.issuer.clone(),
                trust: smime::trust(&signer, &chain, roots, false),
                email_matches,
                not_after: info.not_after,
            });
            security.signature = Verification::Good { fingerprint: info.fingerprint };
            if email_matches {
                collected = Some((signer, chain));
            }
        }
        Some(smime::Checked::UnknownSigner) => security.signature = Verification::UnknownKey { issuer: None },
        Some(smime::Checked::Bad) => security.signature = Verification::Bad,
        None => {}
    }
    let shown = shown.and_then(|inner| pgp_mime::rebuild(&raw, &inner).ok()).unwrap_or(raw);
    (Opened { raw: shown, security: Some(security), subject }, collected)
}

// ---------------------------------------------------------------- envio (CA6)

/// Estado da chave de um destinatário no composer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipientKey {
    pub address: String,
    pub has_key: bool,
    pub post_quantum: bool,
    /// Tem certificado S/MIME que serve para cifrar (US14).
    pub has_certificate: bool,
}

pub fn recipient_status(c: &rusqlite::Connection, addresses: &[String], roots: &[X509]) -> Result<Vec<RecipientKey>, AppError> {
    addresses
        .iter()
        .map(|a| {
            let found = pgp_store::usable_key_for(c, a)?;
            let post_quantum = match &found {
                Some((fp, _)) => pgp_store::get_key(c, fp)?.is_some_and(|k| k.post_quantum),
                None => false,
            };
            let has_certificate = smime_store::usable_cert_for(c, a, roots)?.is_some();
            Ok(RecipientKey { address: a.clone(), has_key: found.is_some(), post_quantum, has_certificate })
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

/// O que o envio S/MIME precisa: o meu certificado e a chave, a cadeia e os certificados dos destinatários.
pub struct SmimeKeys {
    pub cert: X509,
    pub key: openssl::pkey::PKey<openssl::pkey::Private>,
    pub chain: Vec<X509>,
    pub recipients: Vec<X509>,
}

/// Carrega os certificados do envio S/MIME. Cifrar com algum destinatário sem certificado:
/// `MissingRecipientCertificate`.
pub fn smime_keys(c: &rusqlite::Connection, account_id: &str, recipients: &[String], encrypt: bool, roots: &[X509]) -> Result<SmimeKeys, AppError> {
    let settings = pgp_store::crypto_settings(c, account_id)?;
    let fp = settings.smime_cert.ok_or(AppError::NoOwnCertificate)?;
    let mine = smime_store::get_cert(c, &fp)?.ok_or(AppError::NoOwnCertificate)?;
    let key = smime::private_key_from_der(mine.private_key.as_ref().ok_or(AppError::NoOwnCertificate)?)?;
    let mut certs = Vec::new();
    if encrypt {
        for r in recipients {
            certs.push(smime_store::usable_cert_for(c, r, roots)?.ok_or(AppError::MissingRecipientCertificate)?.cert);
        }
        // Também para mim, para eu ler a cópia em Enviados.
        certs.push(mine.cert.clone());
    }
    Ok(SmimeKeys { cert: mine.cert, key, chain: mine.chain, recipients: certs })
}

/// Importa certificados S/MIME (CA1): PKCS#12 com a senha, ou certificados soltos (PEM, DER, `.p7b`).
/// O certificado emitido para um pedido feito aqui ganha a chave do pedido, se o e-mail bater.
pub fn import_smime(c: &mut rusqlite::Connection, data: &[u8], password: Option<&str>) -> Result<Vec<smime_store::Stored>, AppError> {
    if smime::is_pkcs12(data) {
        let bundle = smime::parse_pkcs12(data, password.ok_or(AppError::WrongPassword)?)?;
        let key = smime::private_key_der(&bundle.key)?;
        if smime::public_key_der(&bundle.cert)? != bundle.key.public_key_to_der().map_err(|_| AppError::InvalidInput)? {
            return Err(AppError::InvalidInput);
        }
        return Ok(vec![smime_store::upsert_cert(c, &bundle.cert, &bundle.chain, Some(&key), "pkcs12")?]);
    }
    let certs = smime::parse_certs(data)?;
    // Com e-mail e não emitido por ele mesmo: o certificado de uma pessoa; o resto é cadeia.
    let (people, chain): (Vec<X509>, Vec<X509>) = certs.into_iter().partition(|cert| {
        let info = smime::info(cert);
        !info.emails.is_empty() && !info.self_signed
    });
    if people.is_empty() {
        return Err(AppError::InvalidInput);
    }
    let mut out = Vec::new();
    for cert in people {
        let issued_here = smime_store::request_for_key(c, &smime::public_key_der(&cert)?)?;
        match issued_here {
            Some((id, email, key)) => {
                if !smime::emails(&cert).contains(&email) {
                    return Err(AppError::CertificateEmailMismatch);
                }
                out.push(smime_store::upsert_cert(c, &cert, &chain, Some(&key), "request")?);
                smime_store::delete_request(c, id)?;
            }
            None => out.push(smime_store::upsert_cert(c, &cert, &chain, None, "imported")?),
        }
    }
    Ok(out)
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
            assert!(!recipient_status(c, &["ana@x.com".into()], &[]).unwrap()[0].has_key, "importada, mas ainda não aceita");
            pgp_store::set_acceptance(c, &imported[0].fingerprint, "verified").unwrap();
            let keys = send_keys(c, &a.id, &["ana@x.com".into()], true).unwrap();
            assert_eq!(keys.recipients.len(), 2, "ela e eu");
            assert!(matches!(import(c, openpgp::armored_secret_backup(&mine, "frase longa do backup").unwrap().as_bytes(), None), Err(AppError::WrongPassword)));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn smime_import_matches_requests_and_send_needs_every_certificate() {
        use crate::smime::tests::{issue, person, test_ca};
        use crate::smime::{CertKind, Subject};
        let t = test_db();
        let (ca, ca_key) = test_ca();
        let roots = std::slice::from_ref(&ca);
        t.db.with(|c| {
            let a = account(c, "eu@x.com");
            let req = smime::generate_request(CertKind::Rsa3072, &Subject { name: "Eu", email: "eu@x.com", organization: None, country: None }).unwrap();
            let new = smime_store::NewRequest { account_id: &a.id, email: "eu@x.com", name: "Eu", organization: None, country: None, algorithm: "RSA 3072" };
            smime_store::insert_request(c, &new, &req).unwrap();
            let issued = issue(&req.csr_pem, &ca, &ca_key, 0, 365);
            let pem = format!("{}{}", smime::cert_pem(&issued).unwrap(), smime::cert_pem(&ca).unwrap());
            let mine = import_smime(c, pem.as_bytes(), None).unwrap();
            assert_eq!(mine.len(), 1);
            assert!(mine[0].private_key.is_some(), "a chave do pedido foi para o certificado");
            assert_eq!(mine[0].chain.len(), 1, "a CA que veio junto é a cadeia");
            assert!(smime_store::list_requests(c).unwrap().is_empty());

            let fp = smime::fingerprint(&issued);
            assert!(matches!(smime_keys(c, &a.id, &[], false, roots), Err(AppError::NoOwnCertificate)));
            let s = pgp_store::CryptoSettings { smime_cert: Some(fp), ..Default::default() };
            pgp_store::set_crypto_settings(c, &a.id, &s).unwrap();
            assert!(smime_keys(c, &a.id, &["ana@x.com".into()], false, roots).is_ok(), "só assinar não precisa do dela");
            assert!(matches!(smime_keys(c, &a.id, &["ana@x.com".into()], true, roots), Err(AppError::MissingRecipientCertificate)));
            let (ana, _) = person("ana@x.com", CertKind::P256, &ca, &ca_key);
            import_smime(c, smime::cert_pem(&ana).unwrap().as_bytes(), None).unwrap();
            assert!(recipient_status(c, &["ana@x.com".into()], roots).unwrap()[0].has_certificate);
            assert!(!recipient_status(c, &["ana@x.com".into()], &[]).unwrap()[0].has_certificate, "cadeia não confiável não cifra");
            assert_eq!(smime_keys(c, &a.id, &["ana@x.com".into()], true, roots).unwrap().recipients.len(), 2, "ela e eu");

            // Pedido de um e-mail e certificado emitido para outro.
            let req = smime::generate_request(CertKind::P256, &Subject { name: "Eu", email: "eu@x.com", organization: None, country: None }).unwrap();
            smime_store::insert_request(c, &new, &req).unwrap();
            let other = req.csr_pem.clone();
            let wrong = {
                let r = openssl::x509::X509Req::from_pem(other.as_bytes()).unwrap();
                let mut b = openssl::x509::X509Builder::new().unwrap();
                let mut name = openssl::x509::X509NameBuilder::new().unwrap();
                name.append_entry_by_nid(openssl::nid::Nid::PKCS9_EMAILADDRESS, "golpe@x.com").unwrap();
                b.set_version(2).unwrap();
                b.set_subject_name(&name.build()).unwrap();
                b.set_issuer_name(ca.subject_name()).unwrap();
                b.set_pubkey(&r.public_key().unwrap()).unwrap();
                b.set_not_before(&openssl::asn1::Asn1Time::days_from_now(0).unwrap()).unwrap();
                b.set_not_after(&openssl::asn1::Asn1Time::days_from_now(30).unwrap()).unwrap();
                b.sign(&ca_key, openssl::hash::MessageDigest::sha256()).unwrap();
                b.build()
            };
            assert!(matches!(import_smime(c, &smime::cert_der(&wrong).unwrap(), None), Err(AppError::CertificateEmailMismatch)));

            let p12 = smime::export_pkcs12("Eu", &smime::private_key_from_der(mine[0].private_key.as_ref().unwrap()).unwrap(), &issued, &[], "senha longa do pfx").unwrap();
            assert!(matches!(import_smime(c, &p12, None), Err(AppError::WrongPassword)));
            assert_eq!(import_smime(c, &p12, Some("senha longa do pfx")).unwrap().len(), 1);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn smime_messages_open_verify_and_collect_the_signer() {
        use crate::smime::tests::{person, test_ca};
        use crate::smime::CertKind;
        let (ca, ca_key) = test_ca();
        let roots = std::slice::from_ref(&ca);
        let (ana, ana_key) = person("ana@x.com", CertKind::Rsa3072, &ca, &ca_key);
        let (me, my_key) = person("eu@x.com", CertKind::P256, &ca, &ca_key);
        let part = pgp_mime::canonical(&lettre::message::SinglePart::plain(String::from("Contrato revisado.")).formatted());
        let sig = smime::sign_detached(&part, &ana, &ana_key, std::slice::from_ref(&ca)).unwrap();
        let signed = smime_mime::signed_entity(&part, &sig);
        let inner = signed.bytes();
        let der = smime::encrypt(&inner, std::slice::from_ref(&me)).unwrap();
        let (ct, body, [(name, value)]) = smime_mime::enveloped_entity(&der);
        let header = lettre::message::header::HeaderValue::new(lettre::message::header::HeaderName::new_from_ascii_str(name), value.to_owned());
        let raw = lettre::Message::builder().from("Ana <ana@x.com>".parse().unwrap()).to("eu@x.com".parse().unwrap()).subject("Oi").header(ct).raw_header(header).body(body).unwrap().formatted();

        let mine = vec![smime_store::Stored { cert: me, chain: vec![], private_key: Some(smime::private_key_der(&my_key).unwrap()), origin: "pkcs12".into(), added_at: 0 }];
        let (opened, collected) = read_smime(raw.clone(), "ana@x.com", smime_mime::find(&raw), &[], &mine, roots);
        let sec = opened.security.unwrap();
        assert!(sec.encrypted && sec.decrypted);
        assert!(matches!(sec.signature, Verification::Good { .. }));
        let cert = sec.certificate.unwrap();
        assert_eq!((cert.trust, cert.email_matches), (Trust::Trusted, true));
        assert!(collected.is_some(), "o certificado dela vai para a aba Pessoas");
        let shown = String::from_utf8_lossy(&opened.raw);
        assert!(shown.contains("Contrato revisado.") && !shown.contains("smime.p7s"), "a leitura mostra a parte assinada, sem a assinatura");

        // Sem a minha chave, não abre; de outro remetente, o certificado não é guardado.
        let (closed, _) = read_smime(raw.clone(), "ana@x.com", smime_mime::find(&raw), &[], &[], roots);
        let sec = closed.security.unwrap();
        assert!(sec.encrypted && !sec.decrypted);
        let (_, collected) = read_smime(raw.clone(), "golpe@x.com", smime_mime::find(&raw), &[], &mine, roots);
        assert!(collected.is_none());
        let (untrusted, _) = read_smime(raw.clone(), "ana@x.com", smime_mime::find(&raw), &[], &mine, &[]);
        assert_eq!(untrusted.security.unwrap().certificate.unwrap().trust, Trust::Untrusted);
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
