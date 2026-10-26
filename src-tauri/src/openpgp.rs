//! OpenPGP (US13): gerar, ler e exportar chaves, cifrar, assinar, decifrar e verificar, com o rPGP
//! (RFC 9580; pós-quântico híbrido pela RFC 9980). Nada de criptografia própria: aqui só se escolhem
//! subchaves, algoritmos e formatos. Nenhuma chave, senha ou conteúdo vai para log.

use pgp::composed::{
    ArmorOptions, Deserializable, DetachedSignature, EncryptionCaps, KeyType, Message, MessageBuilder, SecretKeyParamsBuilder, SignedPublicKey,
    SignedSecretKey, SubkeyParamsBuilder,
};
use pgp::crypto::ecc_curve::ECCCurve;
use pgp::crypto::hash::HashAlgorithm;
use pgp::crypto::public_key::PublicKeyAlgorithm;
use pgp::crypto::sym::SymmetricKeyAlgorithm;
use pgp::packet::Signature;
use pgp::ser::Serialize as _;
use pgp::types::{KeyDetails, KeyVersion, Password, VerifyingKey};
use rand::rngs::OsRng;
use serde::Serialize;

use crate::error::AppError;

/// Tipo da chave nova (US13, CA2; D3 da US13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyKind {
    /// Ed25519 + X25519 no formato v4, o que quase todo programa lê (recomendada).
    Curve25519,
    /// RSA 4096 v4, para programas muito antigos.
    Rsa4096,
    /// v6 com ML-DSA-65 + Ed25519 e ML-KEM-768 + X25519 (RFC 9980): experimental.
    PostQuantum,
}

fn pgp_error(context: &'static str) -> impl Fn(pgp::errors::Error) -> AppError {
    move |e| {
        // O texto de erro do rPGP pode citar dados do pacote: só o contexto vai para o log.
        let _ = e;
        tracing::info!(context, "operação OpenPGP recusada");
        AppError::InvalidInput
    }
}

/// `Nome <email>` da identidade.
pub fn user_id(name: &str, email: &str) -> String {
    let name = name.trim().replace(['<', '>'], "");
    if name.is_empty() {
        format!("<{}>", email.trim())
    } else {
        format!("{name} <{}>", email.trim())
    }
}

/// Gera uma chave nova com uma subchave de cifra; a primária assina e certifica.
pub fn generate(kind: KeyKind, name: &str, email: &str) -> Result<SignedSecretKey, AppError> {
    let (version, primary, encryption) = match kind {
        KeyKind::Curve25519 => (KeyVersion::V4, KeyType::Ed25519Legacy, KeyType::ECDH(ECCCurve::Curve25519Legacy)),
        KeyKind::Rsa4096 => (KeyVersion::V4, KeyType::Rsa(4096), KeyType::Rsa(4096)),
        KeyKind::PostQuantum => (KeyVersion::V6, KeyType::MlDsa65Ed25519, KeyType::MlKem768X25519),
    };
    let subkey = SubkeyParamsBuilder::default()
        .version(version)
        .key_type(encryption)
        .can_encrypt(EncryptionCaps::All)
        .build()
        .map_err(|_| AppError::InvalidInput)?;
    let params = SecretKeyParamsBuilder::default()
        .version(version)
        .key_type(primary)
        .can_certify(true)
        .can_sign(true)
        .feature_seipd_v2(version == KeyVersion::V6)
        .primary_user_id(user_id(name, email))
        .preferred_symmetric_algorithms([SymmetricKeyAlgorithm::AES256, SymmetricKeyAlgorithm::AES128].into_iter().collect())
        .preferred_hash_algorithms([HashAlgorithm::Sha512, HashAlgorithm::Sha256].into_iter().collect())
        .subkeys(vec![subkey])
        .build()
        .map_err(|_| AppError::InvalidInput)?;
    params.generate(OsRng).map_err(pgp_error("gerar chave"))
}

/// Uma chave lida de um arquivo ou de um cabeçalho: pública, ou com a parte secreta.
#[derive(Debug, Clone)]
pub enum ParsedKey {
    Public(Box<SignedPublicKey>),
    Secret(Box<SignedSecretKey>),
}

impl ParsedKey {
    pub fn public(&self) -> SignedPublicKey {
        match self {
            ParsedKey::Public(k) => (**k).clone(),
            ParsedKey::Secret(k) => k.to_public_key(),
        }
    }
}

/// Lê chaves em armadura ASCII ou binárias. Chaves com assinaturas próprias inválidas ficam de fora.
pub fn parse_keys(data: &[u8]) -> Result<Vec<ParsedKey>, AppError> {
    let text = String::from_utf8_lossy(data);
    let armored = text.contains("-----BEGIN PGP");
    let mut out = Vec::new();
    if text.contains("PRIVATE KEY BLOCK") || (!armored && SignedSecretKey::from_bytes(data).is_ok()) {
        let keys: Vec<SignedSecretKey> = if armored {
            SignedSecretKey::from_armor_many(data).map_err(pgp_error("ler chave secreta"))?.0.filter_map(Result::ok).collect()
        } else {
            SignedSecretKey::from_bytes_many(data).map(|keys| keys.filter_map(Result::ok).collect()).unwrap_or_default()
        };
        out.extend(keys.into_iter().filter(|k| k.verify_bindings().is_ok()).map(|k| ParsedKey::Secret(Box::new(k))));
    } else {
        let keys: Vec<SignedPublicKey> = if armored {
            SignedPublicKey::from_armor_many(data).map_err(pgp_error("ler chave pública"))?.0.filter_map(Result::ok).collect()
        } else {
            SignedPublicKey::from_bytes_many(data).map(|keys| keys.filter_map(Result::ok).collect()).unwrap_or_default()
        };
        out.extend(keys.into_iter().filter(|k| k.verify_bindings().is_ok()).map(|k| ParsedKey::Public(Box::new(k))));
    }
    if out.is_empty() {
        return Err(AppError::InvalidInput);
    }
    Ok(out)
}

/// A parte secreta está protegida por senha?
pub fn is_locked(key: &SignedSecretKey) -> bool {
    matches!(key.primary_key.secret_params(), pgp::types::SecretParams::Encrypted(_))
        || key.secret_subkeys.iter().any(|s| matches!(s.key.secret_params(), pgp::types::SecretParams::Encrypted(_)))
}

/// Tira a senha da chave importada: ela passa a ser protegida pelo banco cifrado (e pela senha do
/// Epistula, US10). Senha errada: `WrongPassword`.
pub fn unlock(mut key: SignedSecretKey, passphrase: &str) -> Result<SignedSecretKey, AppError> {
    if !is_locked(&key) {
        return Ok(key);
    }
    let pw = Password::from(passphrase);
    key.primary_key.remove_password(&pw).map_err(|_| AppError::WrongPassword)?;
    for sub in &mut key.secret_subkeys {
        sub.key.remove_password(&pw).map_err(|_| AppError::WrongPassword)?;
    }
    Ok(key)
}

/// Bytes para guardar no banco (formato binário OpenPGP).
pub fn secret_bytes(key: &SignedSecretKey) -> Result<Vec<u8>, AppError> {
    key.to_bytes().map_err(pgp_error("gravar chave"))
}

pub fn public_bytes(key: &SignedPublicKey) -> Result<Vec<u8>, AppError> {
    key.to_bytes().map_err(pgp_error("gravar chave"))
}

pub fn secret_from_bytes(bytes: &[u8]) -> Result<SignedSecretKey, AppError> {
    SignedSecretKey::from_bytes(bytes).map_err(pgp_error("ler chave guardada"))
}

pub fn public_from_bytes(bytes: &[u8]) -> Result<SignedPublicKey, AppError> {
    SignedPublicKey::from_bytes(bytes).map_err(pgp_error("ler chave guardada"))
}

/// A chave pública em armadura ASCII, para exportar ou anexar.
pub fn armored_public(key: &SignedPublicKey) -> Result<String, AppError> {
    key.to_armored_string(ArmorOptions::default()).map_err(pgp_error("exportar chave"))
}

/// Backup da chave secreta, sempre protegido por senha (US13, regra 2).
pub fn armored_secret_backup(key: &SignedSecretKey, passphrase: &str) -> Result<String, AppError> {
    if passphrase.chars().count() < crate::lock::MIN_PASSWORD_CHARS {
        return Err(AppError::WeakPassword);
    }
    let mut copy = key.clone();
    let pw = Password::from(passphrase);
    copy.primary_key.set_password(OsRng, &pw).map_err(pgp_error("proteger backup"))?;
    for sub in &mut copy.secret_subkeys {
        sub.key.set_password(OsRng, &pw).map_err(pgp_error("proteger backup"))?;
    }
    copy.to_armored_string(ArmorOptions::default()).map_err(pgp_error("exportar backup"))
}

// ---------------------------------------------------------------- o que a tela mostra de uma chave

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyInfo {
    /// Impressão digital em hexadecimal maiúsculo, sem espaços.
    pub fingerprint: String,
    pub version: u8,
    /// "Ed25519 / X25519", "RSA 4096", "ML-DSA-65 + Ed25519 / ML-KEM-768 + X25519".
    pub algorithm: String,
    pub post_quantum: bool,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub user_ids: Vec<String>,
    pub emails: Vec<String>,
    pub can_encrypt: bool,
    pub revoked: bool,
}

fn algorithm_name(alg: PublicKeyAlgorithm, bits: Option<usize>) -> String {
    use PublicKeyAlgorithm as A;
    match alg {
        A::RSA | A::RSAEncrypt | A::RSASign => bits.map_or("RSA".into(), |b| format!("RSA {b}")),
        A::EdDSALegacy | A::Ed25519 => "Ed25519".into(),
        A::ECDH => "X25519".into(),
        A::X25519 => "X25519".into(),
        A::Ed448 => "Ed448".into(),
        A::X448 => "X448".into(),
        A::ECDSA => "ECDSA".into(),
        A::MlDsa65Ed25519 => "ML-DSA-65 + Ed25519".into(),
        A::MlKem768X25519 => "ML-KEM-768 + X25519".into(),
        other => format!("{other:?}"),
    }
}

fn is_post_quantum(alg: PublicKeyAlgorithm) -> bool {
    matches!(alg, PublicKeyAlgorithm::MlDsa65Ed25519 | PublicKeyAlgorithm::MlKem768X25519 | PublicKeyAlgorithm::MlDsa87Ed448 | PublicKeyAlgorithm::MlKem1024X448)
}

fn rsa_bits(params: &pgp::types::PublicParams) -> Option<usize> {
    match params {
        pgp::types::PublicParams::RSA(p) => Some(rsa::traits::PublicKeyParts::n(&p.key).bits()),
        _ => None,
    }
}

/// O e-mail de uma identidade `Nome <email>`, em minúsculas.
pub fn email_of(uid: &str) -> Option<String> {
    let inner = match (uid.rfind('<'), uid.rfind('>')) {
        (Some(a), Some(b)) if a < b => &uid[a + 1..b],
        _ => uid,
    };
    let email = inner.trim().to_lowercase();
    (email.contains('@') && !email.contains(char::is_whitespace)).then_some(email)
}

/// Subchave de cifra válida (sinalizada para cifrar comunicação).
fn encryption_subkey(key: &SignedPublicKey) -> Option<&pgp::composed::SignedPublicSubKey> {
    key.public_subkeys.iter().find(|s| s.signatures.iter().any(|sig| sig.key_flags().encrypt_comms()))
}

pub fn info(key: &SignedPublicKey) -> KeyInfo {
    let user_ids: Vec<String> = key.details.users.iter().map(|u| String::from_utf8_lossy(u.id.id()).into_owned()).collect();
    let mut emails: Vec<String> = user_ids.iter().filter_map(|u| email_of(u)).collect();
    emails.dedup();
    let created_at = key.primary_key.created_at().as_secs() as i64;
    // A validade fica na assinatura própria mais recente da identidade (ou na direta).
    let expires_at = key
        .details
        .users
        .iter()
        .flat_map(|u| u.signatures.iter())
        .chain(key.details.direct_signatures.iter())
        .filter(|s| s.created().is_some())
        .max_by_key(|s| s.created().map(|t| t.as_secs()))
        .and_then(Signature::key_expiration_time)
        .map(|d| created_at + d.as_secs() as i64);
    let primary_alg = algorithm_name(key.primary_key.algorithm(), rsa_bits(key.primary_key.public_params()));
    let enc = encryption_subkey(key);
    let algorithm = match enc {
        Some(sub) => {
            let name = algorithm_name(sub.key.algorithm(), rsa_bits(sub.key.public_params()));
            if name == primary_alg { name } else { format!("{primary_alg} / {name}") }
        }
        None => primary_alg,
    };
    KeyInfo {
        fingerprint: format!("{:X}", key.fingerprint()),
        version: match key.version() {
            KeyVersion::V6 => 6,
            KeyVersion::V4 => 4,
            _ => 3,
        },
        algorithm,
        post_quantum: is_post_quantum(key.primary_key.algorithm()) || enc.is_some_and(|s| is_post_quantum(s.key.algorithm())),
        created_at,
        expires_at,
        user_ids,
        emails,
        can_encrypt: enc.is_some(),
        revoked: !key.details.revocation_signatures.is_empty(),
    }
}

// ---------------------------------------------------------------- cifrar, assinar, decifrar, verificar

/// Cifra para os destinatários (e para quem envia, para ler a própria cópia) e assina. Todas as
/// chaves pós-quânticas: SEIPDv2 (AEAD); senão SEIPDv1, que todo programa lê.
pub fn encrypt(data: &[u8], recipients: &[SignedPublicKey], signer: Option<&SignedSecretKey>) -> Result<String, AppError> {
    let subkeys: Vec<_> = recipients.iter().map(encryption_subkey).collect::<Option<Vec<_>>>().ok_or(AppError::InvalidInput)?;
    if subkeys.is_empty() {
        return Err(AppError::InvalidInput);
    }
    let all_v6 = recipients.iter().all(|k| k.version() == KeyVersion::V6);
    let mut rng = OsRng;
    let mut builder = MessageBuilder::from_bytes("", data.to_vec());
    if let Some(key) = signer {
        builder.sign(&key.primary_key, Password::empty(), HashAlgorithm::Sha512);
    }
    if all_v6 {
        let mut b = builder.seipd_v2(&mut rng, SymmetricKeyAlgorithm::AES256, pgp::crypto::aead::AeadAlgorithm::Ocb, pgp::crypto::aead::ChunkSize::default());
        for sub in &subkeys {
            b.encrypt_to_key(&mut rng, &sub.key).map_err(pgp_error("cifrar"))?;
        }
        b.to_armored_string(rng, ArmorOptions::default()).map_err(pgp_error("cifrar"))
    } else {
        let mut b = builder.seipd_v1(&mut rng, SymmetricKeyAlgorithm::AES256);
        for sub in &subkeys {
            b.encrypt_to_key(&mut rng, &sub.key).map_err(pgp_error("cifrar"))?;
        }
        b.to_armored_string(rng, ArmorOptions::default()).map_err(pgp_error("cifrar"))
    }
}

/// Assinatura destacada (PGP/MIME `multipart/signed`), em armadura.
pub fn sign_detached(data: &[u8], signer: &SignedSecretKey) -> Result<String, AppError> {
    let sig = DetachedSignature::sign_binary_data(OsRng, &signer.primary_key, &Password::empty(), HashAlgorithm::Sha512, data)
        .map_err(pgp_error("assinar"))?;
    sig.to_armored_string(ArmorOptions::default()).map_err(pgp_error("assinar"))
}

/// Resultado da verificação de uma assinatura.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Verification {
    /// Sem assinatura.
    Unsigned,
    /// Confere com esta chave (impressão digital).
    Good { fingerprint: String },
    /// Assinada por uma chave que não está aqui (impressão digital ou ID, quando houver).
    UnknownKey { issuer: Option<String> },
    /// A chave existe, mas a assinatura não confere: conteúdo alterado.
    Bad,
}

fn issuer_of(sig: &Signature) -> Option<String> {
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02X}")).collect::<String>();
    sig.issuer_fingerprint().first().map(|f| format!("{f:X}")).or_else(|| sig.issuer_key_id().first().map(|k| hex(k.as_ref())))
}

fn issuer_matches(issuer: &str, key: &SignedPublicKey) -> bool {
    let fp = format!("{:X}", key.fingerprint());
    fp == issuer || fp.ends_with(issuer) || key.public_subkeys.iter().any(|s| {
        let sfp = format!("{:X}", s.key.fingerprint());
        sfp == issuer || sfp.ends_with(issuer)
    })
}

/// Chaves que podem ter assinado: a primária e as subchaves de cada chave conhecida.
fn verifiers(keys: &[SignedPublicKey]) -> Vec<(&SignedPublicKey, &dyn VerifyingKey)> {
    let mut out: Vec<(&SignedPublicKey, &dyn VerifyingKey)> = Vec::new();
    for k in keys {
        out.push((k, &k.primary_key));
        for s in &k.public_subkeys {
            out.push((k, &s.key));
        }
    }
    out
}

pub fn verify_detached(data: &[u8], armored_sig: &[u8], keys: &[SignedPublicKey]) -> Verification {
    let Ok((sig, _)) = DetachedSignature::from_armor_single(armored_sig).or_else(|_| DetachedSignature::from_bytes(armored_sig).map(|s| (s, Default::default()))) else {
        return Verification::Bad;
    };
    let issuer = issuer_of(&sig.signature);
    for key in keys {
        // `DetachedSignature::verify` pede um tipo concreto: a primária e cada subchave.
        let good = sig.verify(&key.primary_key, data).is_ok() || key.public_subkeys.iter().any(|s| sig.verify(&s.key, data).is_ok());
        if good {
            return Verification::Good { fingerprint: format!("{:X}", key.fingerprint()) };
        }
    }
    match &issuer {
        Some(i) if keys.iter().any(|k| issuer_matches(i, k)) => Verification::Bad,
        _ => Verification::UnknownKey { issuer },
    }
}

/// Mensagem decifrada: o conteúdo e a verificação da assinatura de dentro.
#[derive(Debug)]
pub struct Decrypted {
    pub data: Vec<u8>,
    pub verification: Verification,
    /// Cifrada com algoritmo pós-quântico (alguma chave da mensagem era).
    pub post_quantum: bool,
}

/// Decifra com a primeira chave secreta que abrir; verifica a assinatura com as públicas dadas.
pub fn decrypt(data: &[u8], secrets: &[SignedSecretKey], signers: &[SignedPublicKey]) -> Result<Decrypted, AppError> {
    let parse = || {
        Message::from_armor(data).map(|(m, _)| m).or_else(|_| Message::from_bytes(data))
    };
    for key in secrets {
        let Ok(msg) = parse() else { return Err(AppError::InvalidInput) };
        let Ok(mut inner) = msg.decrypt(&Password::empty(), key) else { continue };
        if inner.is_compressed() {
            inner = inner.decompress().map_err(pgp_error("descomprimir"))?;
        }
        let plain = inner.as_data_vec().map_err(|_| AppError::InvalidInput)?;
        let verification = if inner.is_signed() {
            let found = verifiers(signers).into_iter().find_map(|(k, v)| inner.verify(v).ok().map(|_| format!("{:X}", k.fingerprint())));
            match found {
                Some(fingerprint) => Verification::Good { fingerprint },
                None => {
                    let issuer = match &inner {
                        Message::Signed { reader, .. } => reader.signature(0).and_then(issuer_of),
                        _ => None,
                    };
                    match &issuer {
                        Some(i) if signers.iter().any(|k| issuer_matches(i, k)) => Verification::Bad,
                        _ => Verification::UnknownKey { issuer },
                    }
                }
            }
        } else {
            Verification::Unsigned
        };
        let post_quantum = key.secret_subkeys.iter().any(|s| is_post_quantum(s.key.algorithm()));
        return Ok(Decrypted { data: plain, verification, post_quantum });
    }
    Err(AppError::NoKeyForMessage)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(kind: KeyKind, email: &str) -> (SignedSecretKey, SignedPublicKey) {
        let secret = generate(kind, "Teste", email).unwrap();
        let public = secret.to_public_key();
        (secret, public)
    }

    #[test]
    fn curve25519_key_roundtrips_and_reads_as_expected() {
        let (secret, public) = pair(KeyKind::Curve25519, "ana@exemplo.com");
        let i = info(&public);
        assert_eq!((i.version, i.post_quantum, i.can_encrypt), (4, false, true));
        assert_eq!(i.algorithm, "Ed25519 / X25519");
        assert_eq!(i.emails, vec!["ana@exemplo.com".to_owned()]);
        assert_eq!(i.fingerprint.len(), 40);
        let stored = secret_from_bytes(&secret_bytes(&secret).unwrap()).unwrap();
        assert_eq!(format!("{:X}", stored.fingerprint()), i.fingerprint);
        let armored = armored_public(&public).unwrap();
        assert!(armored.starts_with("-----BEGIN PGP PUBLIC KEY BLOCK-----"));
        let parsed = parse_keys(armored.as_bytes()).unwrap();
        assert!(matches!(&parsed[..], [ParsedKey::Public(_)]));
    }

    #[test]
    fn encrypts_signs_decrypts_and_verifies() {
        let (ana_s, ana_p) = pair(KeyKind::Curve25519, "ana@exemplo.com");
        let (bia_s, bia_p) = pair(KeyKind::Curve25519, "bia@exemplo.com");
        let text = b"Contrato confidencial, versao 3.";
        let armored = encrypt(text, &[bia_p.clone(), ana_p.clone()], Some(&ana_s)).unwrap();
        assert!(armored.contains("BEGIN PGP MESSAGE"));
        assert!(!armored.contains("confidencial"));

        let out = decrypt(armored.as_bytes(), std::slice::from_ref(&bia_s), std::slice::from_ref(&ana_p)).unwrap();
        assert_eq!(out.data, text);
        assert_eq!(out.verification, Verification::Good { fingerprint: format!("{:X}", ana_p.fingerprint()) });
        assert!(decrypt(armored.as_bytes(), &[ana_s], &[]).is_ok(), "quem envia lê a própria cópia");

        let unknown = decrypt(armored.as_bytes(), std::slice::from_ref(&bia_s), &[]).unwrap();
        assert!(matches!(unknown.verification, Verification::UnknownKey { .. }));

        let (stranger, _) = pair(KeyKind::Curve25519, "x@exemplo.com");
        assert!(matches!(decrypt(armored.as_bytes(), &[stranger], &[]), Err(AppError::NoKeyForMessage)));
    }

    #[test]
    fn detached_signature_detects_tampering() {
        let (s, p) = pair(KeyKind::Curve25519, "ana@exemplo.com");
        let sig = sign_detached(b"Content-Type: text/plain\r\n\r\nOi", &s).unwrap();
        assert!(matches!(verify_detached(b"Content-Type: text/plain\r\n\r\nOi", sig.as_bytes(), std::slice::from_ref(&p)), Verification::Good { .. }));
        assert_eq!(verify_detached(b"Content-Type: text/plain\r\n\r\nTchau", sig.as_bytes(), std::slice::from_ref(&p)), Verification::Bad);
        assert!(matches!(verify_detached(b"x", sig.as_bytes(), &[]), Verification::UnknownKey { .. }));
    }

    #[test]
    fn post_quantum_key_roundtrips() {
        let (s, p) = pair(KeyKind::PostQuantum, "pq@exemplo.com");
        let i = info(&p);
        assert_eq!((i.version, i.post_quantum, i.can_encrypt), (6, true, true));
        let armored = encrypt(b"segredo de longo prazo", std::slice::from_ref(&p), Some(&s)).unwrap();
        let out = decrypt(armored.as_bytes(), std::slice::from_ref(&s), std::slice::from_ref(&p)).unwrap();
        assert_eq!(out.data, b"segredo de longo prazo");
        assert!(out.post_quantum);
        assert!(matches!(out.verification, Verification::Good { .. }));
    }

    #[test]
    fn backup_is_protected_and_unlocks_with_its_passphrase() {
        let (s, _) = pair(KeyKind::Curve25519, "ana@exemplo.com");
        assert!(matches!(armored_secret_backup(&s, "curta"), Err(AppError::WeakPassword)));
        let backup = armored_secret_backup(&s, "frase longa do backup").unwrap();
        let parsed = parse_keys(backup.as_bytes()).unwrap();
        let ParsedKey::Secret(locked) = &parsed[0] else { panic!("esperava a chave secreta") };
        assert!(is_locked(locked));
        assert!(matches!(unlock((**locked).clone(), "senha errada"), Err(AppError::WrongPassword)));
        let open = unlock((**locked).clone(), "frase longa do backup").unwrap();
        assert!(!is_locked(&open));
        assert_eq!(format!("{:X}", open.fingerprint()), format!("{:X}", s.fingerprint()));
    }

    #[test]
    fn emails_come_from_user_ids() {
        assert_eq!(email_of("Ana Ribeiro <Ana@Lumen.com.br>").as_deref(), Some("ana@lumen.com.br"));
        assert_eq!(email_of("ana@lumen.com.br").as_deref(), Some("ana@lumen.com.br"));
        assert_eq!(email_of("Só um nome"), None);
        assert_eq!(user_id("Ana <x>", "ana@x.com"), "Ana x <ana@x.com>");
    }
}
