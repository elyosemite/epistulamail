//! S/MIME (US14): pedido de certificado (CSR), PKCS#12, certificados X.509, CMS (cifrar, assinar,
//! verificar, decifrar) e a cadeia até as raízes do Windows (D6 da US13). Tudo pelo OpenSSL que o
//! SQLCipher já embute (D2 da US13). Nenhuma criptografia própria. Logs: só o contexto, nunca
//! titulares, e-mails, chaves ou conteúdo.

use foreign_types::{ForeignType, ForeignTypeRef};
use openssl::asn1::Asn1Time;
use openssl::cms::{CMSOptions, CmsContentInfo, CmsContentInfoRef};
use openssl::ec::{EcGroup, EcKey};
use openssl::error::ErrorStack;
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkcs12::Pkcs12;
use openssl::pkcs7::Pkcs7;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::stack::Stack;
use openssl::symm::Cipher;
use openssl::x509::extension::{ExtendedKeyUsage, KeyUsage, SubjectAlternativeName};
use openssl::x509::store::X509StoreBuilder;
use openssl::x509::{X509NameBuilder, X509NameRef, X509PurposeId, X509Ref, X509ReqBuilder, X509StoreContext, X509VerifyResult, X509};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::AppError;

fn ssl_error(context: &'static str) -> impl Fn(ErrorStack) -> AppError {
    move |e| {
        // A pilha de erros do OpenSSL pode citar campos do certificado: só o contexto vai para o log.
        let _ = e;
        tracing::info!(context, "operação S/MIME recusada");
        AppError::InvalidInput
    }
}

// ---------------------------------------------------------------- pedido de certificado (CA1)

/// Algoritmo do par de chaves do pedido. RSA 3072 é o que as CAs mais aceitam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CertKind {
    Rsa3072,
    P256,
}

impl CertKind {
    pub fn label(self) -> &'static str {
        match self {
            CertKind::Rsa3072 => "RSA 3072",
            CertKind::P256 => "ECDSA P-256",
        }
    }
}

/// Dados de quem pede: o e-mail é o da conta.
pub struct Subject<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub organization: Option<&'a str>,
    pub country: Option<&'a str>,
}

/// O par de chaves gerado aqui e o CSR para levar à CA.
pub struct Request {
    /// Chave privada em PKCS#8 (DER).
    pub private_key: Zeroizing<Vec<u8>>,
    /// Chave pública (SubjectPublicKeyInfo, DER), para casar o certificado emitido.
    pub public_key: Vec<u8>,
    pub csr_pem: String,
}

fn clean(text: &str, max: usize) -> String {
    text.chars().filter(|c| !c.is_control()).take(max).collect::<String>().trim().to_owned()
}

pub fn generate_request(kind: CertKind, subject: &Subject<'_>) -> Result<Request, AppError> {
    let key: PKey<Private> = match kind {
        CertKind::Rsa3072 => PKey::from_rsa(Rsa::generate(3072).map_err(ssl_error("gerar RSA"))?),
        CertKind::P256 => {
            let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).map_err(ssl_error("curva"))?;
            PKey::from_ec_key(EcKey::generate(&group).map_err(ssl_error("gerar EC"))?)
        }
    }
    .map_err(ssl_error("chave"))?;
    let email = subject.email.trim().to_lowercase();
    let mut name = X509NameBuilder::new().map_err(ssl_error("nome"))?;
    let common = clean(subject.name, 64);
    name.append_entry_by_nid(Nid::COMMONNAME, if common.is_empty() { &email } else { &common }).map_err(ssl_error("nome"))?;
    if let Some(org) = subject.organization.map(|o| clean(o, 64)).filter(|o| !o.is_empty()) {
        name.append_entry_by_nid(Nid::ORGANIZATIONNAME, &org).map_err(ssl_error("organização"))?;
    }
    if let Some(country) = subject.country.map(|c| clean(c, 2).to_uppercase()).filter(|c| c.len() == 2 && c.chars().all(|ch| ch.is_ascii_alphabetic())) {
        name.append_entry_by_nid(Nid::COUNTRYNAME, &country).map_err(ssl_error("país"))?;
    }
    name.append_entry_by_nid(Nid::PKCS9_EMAILADDRESS, &email).map_err(ssl_error("e-mail"))?;
    let name = name.build();

    let mut req = X509ReqBuilder::new().map_err(ssl_error("pedido"))?;
    req.set_version(0).map_err(ssl_error("pedido"))?;
    req.set_subject_name(&name).map_err(ssl_error("pedido"))?;
    req.set_pubkey(&key).map_err(ssl_error("pedido"))?;
    let mut extensions = Stack::new().map_err(ssl_error("extensões"))?;
    let context = req.x509v3_context(None);
    extensions.push(SubjectAlternativeName::new().email(&email).build(&context).map_err(ssl_error("SAN"))?).map_err(ssl_error("SAN"))?;
    let mut usage = KeyUsage::new();
    usage.critical().digital_signature().non_repudiation();
    match kind {
        CertKind::Rsa3072 => usage.key_encipherment(),
        CertKind::P256 => usage.key_agreement(),
    };
    extensions.push(usage.build().map_err(ssl_error("uso"))?).map_err(ssl_error("uso"))?;
    extensions.push(ExtendedKeyUsage::new().email_protection().build().map_err(ssl_error("uso"))?).map_err(ssl_error("uso"))?;
    req.add_extensions(&extensions).map_err(ssl_error("extensões"))?;
    req.sign(&key, MessageDigest::sha256()).map_err(ssl_error("assinar pedido"))?;
    let csr = req.build();
    Ok(Request {
        private_key: Zeroizing::new(key.private_key_to_pkcs8().map_err(ssl_error("PKCS#8"))?),
        public_key: key.public_key_to_der().map_err(ssl_error("chave pública"))?,
        csr_pem: String::from_utf8(csr.to_pem().map_err(ssl_error("PEM"))?).map_err(|_| AppError::InvalidInput)?,
    })
}

pub fn private_key_from_der(der: &[u8]) -> Result<PKey<Private>, AppError> {
    PKey::private_key_from_pkcs8(der).map_err(ssl_error("ler chave"))
}

pub fn private_key_der(key: &PKey<Private>) -> Result<Zeroizing<Vec<u8>>, AppError> {
    Ok(Zeroizing::new(key.private_key_to_pkcs8().map_err(ssl_error("PKCS#8"))?))
}

/// A chave pública do certificado é a do par (para casar com um pedido ou com a chave de um PKCS#12).
pub fn public_key_der(cert: &X509Ref) -> Result<Vec<u8>, AppError> {
    cert.public_key().and_then(|k| k.public_key_to_der()).map_err(ssl_error("chave pública"))
}

// ---------------------------------------------------------------- certificados

/// Lê certificados de um arquivo: PEM (um ou vários), DER, ou PKCS#7 só com certificados (`.p7b`).
pub fn parse_certs(data: &[u8]) -> Result<Vec<X509>, AppError> {
    let text = String::from_utf8_lossy(data);
    if text.contains("-----BEGIN CERTIFICATE-----") {
        return X509::stack_from_pem(data).map_err(ssl_error("ler PEM"));
    }
    if text.contains("-----BEGIN PKCS7-----") {
        let p7 = Pkcs7::from_pem(data).map_err(ssl_error("ler PKCS#7"))?;
        return Ok(p7.signed().and_then(|s| s.certificates()).map(|st| st.iter().map(X509Ref::to_owned).collect()).unwrap_or_default());
    }
    if let Ok(cert) = X509::from_der(data) {
        return Ok(vec![cert]);
    }
    let p7 = Pkcs7::from_der(data).map_err(ssl_error("ler certificado"))?;
    Ok(p7.signed().and_then(|s| s.certificates()).map(|st| st.iter().map(X509Ref::to_owned).collect()).unwrap_or_default())
}

/// Um arquivo PKCS#12 (`.p12`, `.pfx`): a chave, o certificado e a cadeia. Senha errada: `WrongPassword`.
pub struct Bundle {
    pub key: PKey<Private>,
    pub cert: X509,
    pub chain: Vec<X509>,
}

pub fn is_pkcs12(data: &[u8]) -> bool {
    Pkcs12::from_der(data).is_ok()
}

pub fn parse_pkcs12(data: &[u8], password: &str) -> Result<Bundle, AppError> {
    let p12 = Pkcs12::from_der(data).map_err(ssl_error("ler PKCS#12"))?;
    let parsed = p12.parse2(password).map_err(|_| AppError::WrongPassword)?;
    let (Some(key), Some(cert)) = (parsed.pkey, parsed.cert) else { return Err(AppError::InvalidInput) };
    let chain = parsed.ca.map(|st| st.into_iter().collect()).unwrap_or_default();
    Ok(Bundle { key, cert, chain })
}

/// Meu certificado com a chave, protegido por senha (8 ou mais), para outro computador ou programa.
pub fn export_pkcs12(name: &str, key: &PKey<Private>, cert: &X509Ref, chain: &[X509], password: &str) -> Result<Vec<u8>, AppError> {
    if password.chars().count() < crate::lock::MIN_PASSWORD_CHARS {
        return Err(AppError::WeakPassword);
    }
    let mut ca = Stack::new().map_err(ssl_error("cadeia"))?;
    for c in chain {
        ca.push(c.clone()).map_err(ssl_error("cadeia"))?;
    }
    let mut builder = Pkcs12::builder();
    builder.name(&clean(name, 64)).pkey(key).cert(cert);
    if !chain.is_empty() {
        builder.ca(ca);
    }
    // OpenSSL 3: PBKDF2 com AES-256-CBC e MAC com SHA-256 por padrão.
    let p12 = builder.build2(password).map_err(ssl_error("montar PKCS#12"))?;
    p12.to_der().map_err(ssl_error("PKCS#12"))
}

pub fn cert_pem(cert: &X509Ref) -> Result<String, AppError> {
    String::from_utf8(cert.to_pem().map_err(ssl_error("PEM"))?).map_err(|_| AppError::InvalidInput)
}

pub fn cert_der(cert: &X509Ref) -> Result<Vec<u8>, AppError> {
    cert.to_der().map_err(ssl_error("DER"))
}

pub fn chain_pem(chain: &[X509]) -> String {
    chain.iter().filter_map(|c| cert_pem(c).ok()).collect()
}

pub fn chain_from_pem(pem: &str) -> Vec<X509> {
    if pem.trim().is_empty() {
        return Vec::new();
    }
    X509::stack_from_pem(pem.as_bytes()).unwrap_or_default()
}

/// O que o gerenciador mostra de um certificado (CA2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertInfo {
    /// SHA-256 do certificado, em hexadecimal maiúsculo.
    pub fingerprint: String,
    pub subject: String,
    pub organization: Option<String>,
    pub emails: Vec<String>,
    pub issuer: String,
    pub serial: String,
    pub algorithm: String,
    pub not_before: i64,
    pub not_after: i64,
    pub can_sign: bool,
    pub can_encrypt: bool,
    /// Emitido por ele mesmo (CA de teste, raiz ou certificado caseiro).
    pub self_signed: bool,
}

fn entry(name: &X509NameRef, nid: Nid) -> Option<String> {
    name.entries_by_nid(nid).next().and_then(|e| e.data().to_string().ok())
}

fn unix(time: &openssl::asn1::Asn1TimeRef) -> i64 {
    Asn1Time::from_unix(0).ok().and_then(|epoch| epoch.diff(time).ok()).map(|d| i64::from(d.days) * 86_400 + i64::from(d.secs)).unwrap_or(0)
}

/// Uso das chaves (keyUsage e extendedKeyUsage). Sem a extensão, o OpenSSL devolve "tudo permitido".
fn usage(cert: &X509Ref) -> (bool, bool) {
    // SAFETY: as duas funções só leem o certificado, que continua vivo pela referência.
    let (ku, xku) = unsafe { (openssl_sys::X509_get_key_usage(cert.as_ptr()), openssl_sys::X509_get_extended_key_usage(cert.as_ptr())) };
    let smime = xku & openssl_sys::XKU_SMIME != 0;
    let sign = ku & (openssl_sys::X509v3_KU_DIGITAL_SIGNATURE | openssl_sys::X509v3_KU_NON_REPUDIATION) != 0;
    let encrypt = ku & (openssl_sys::X509v3_KU_KEY_ENCIPHERMENT | openssl_sys::X509v3_KU_KEY_AGREEMENT) != 0;
    (smime && sign, smime && encrypt)
}

fn algorithm(cert: &X509Ref) -> String {
    match cert.public_key() {
        Ok(k) => match k.id() {
            openssl::pkey::Id::RSA => format!("RSA {}", k.bits()),
            openssl::pkey::Id::EC => match k.ec_key().ok().and_then(|ec| ec.group().curve_name()) {
                Some(Nid::X9_62_PRIME256V1) => "ECDSA P-256".into(),
                Some(Nid::SECP384R1) => "ECDSA P-384".into(),
                _ => "ECDSA".into(),
            },
            openssl::pkey::Id::ED25519 => "Ed25519".into(),
            _ => "Outro".into(),
        },
        Err(_) => "Desconhecido".into(),
    }
}

pub fn emails(cert: &X509Ref) -> Vec<String> {
    let mut out: Vec<String> = cert.subject_alt_names().map(|names| names.iter().filter_map(|n| n.email().map(|e| e.trim().to_lowercase())).collect()).unwrap_or_default();
    if let Some(e) = entry(cert.subject_name(), Nid::PKCS9_EMAILADDRESS) {
        out.push(e.trim().to_lowercase());
    }
    out.sort();
    out.dedup();
    out.retain(|e| e.contains('@'));
    out
}

pub fn fingerprint(cert: &X509Ref) -> String {
    cert.digest(MessageDigest::sha256()).map(|d| d.iter().map(|b| format!("{b:02X}")).collect()).unwrap_or_default()
}

pub fn info(cert: &X509Ref) -> CertInfo {
    let subject = cert.subject_name();
    let issuer = cert.issuer_name();
    let (can_sign, can_encrypt) = usage(cert);
    let emails = emails(cert);
    CertInfo {
        fingerprint: fingerprint(cert),
        subject: entry(subject, Nid::COMMONNAME).or_else(|| emails.first().cloned()).unwrap_or_default(),
        organization: entry(subject, Nid::ORGANIZATIONNAME),
        emails,
        issuer: entry(issuer, Nid::ORGANIZATIONNAME).or_else(|| entry(issuer, Nid::COMMONNAME)).unwrap_or_default(),
        serial: cert.serial_number().to_bn().and_then(|b| b.to_hex_str().map(|s| s.to_string())).unwrap_or_default(),
        algorithm: algorithm(cert),
        not_before: unix(cert.not_before()),
        not_after: unix(cert.not_after()),
        can_sign,
        can_encrypt,
        self_signed: cert.issued(cert) == X509VerifyResult::OK,
    }
}

// ---------------------------------------------------------------- confiança (D6)

/// Se a cadeia chega a uma raiz confiável. A revogação (OCSP, CRL) não é consultada: D1 da US14.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Trust {
    Trusted,
    Expired,
    Untrusted,
}

/// Raízes de confiança do usuário no Windows (inclui as da máquina), lidas uma vez por execução.
pub fn system_roots() -> &'static [X509] {
    static ROOTS: std::sync::OnceLock<Vec<X509>> = std::sync::OnceLock::new();
    ROOTS.get_or_init(|| {
        #[cfg(windows)]
        {
            match schannel::cert_store::CertStore::open_current_user("ROOT") {
                Ok(store) => store.certs().filter_map(|c| X509::from_der(c.to_der()).ok()).collect(),
                Err(_) => {
                    tracing::warn!("raízes do Windows indisponíveis");
                    Vec::new()
                }
            }
        }
        #[cfg(not(windows))]
        {
            load_system_roots()
        }
    })
}

/// Raízes de confiança do sistema fora do Windows: o bundle de CAs da distribuição, que o OpenSSL
/// também usa (`SSL_CERT_FILE` e os caminhos comuns de Debian, Fedora e macOS).
#[cfg(not(windows))]
fn load_system_roots() -> Vec<X509> {
    let mut paths = Vec::new();
    if let Ok(path) = std::env::var("SSL_CERT_FILE") {
        paths.push(std::path::PathBuf::from(path));
    }
    for path in [
        "/etc/ssl/certs/ca-certificates.crt",
        "/etc/pki/tls/certs/ca-bundle.crt",
        "/etc/ssl/cert.pem",
        "/etc/ssl/certs/ca-bundle.crt",
    ] {
        paths.push(std::path::PathBuf::from(path));
    }
    for path in paths {
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let Ok(certs) = X509::stack_from_pem(&bytes) else { continue };
        if !certs.is_empty() {
            return certs;
        }
    }
    tracing::warn!("raízes de confiança do sistema indisponíveis");
    Vec::new()
}

/// Confere a cadeia do certificado (com os intermediários dados) até uma das raízes, para e-mail.
pub fn trust(cert: &X509Ref, intermediates: &[X509], roots: &[X509], encrypt: bool) -> Trust {
    let check = || -> Result<Trust, ErrorStack> {
        let mut builder = X509StoreBuilder::new()?;
        for r in roots {
            builder.add_cert(r.clone())?;
        }
        // Para cifrar, o uso é conferido em `info` (o propósito do OpenSSL exige keyEncipherment e
        // recusaria os certificados ECDSA, que usam keyAgreement).
        if !encrypt {
            builder.set_purpose(X509PurposeId::SMIME_SIGN)?;
        }
        let store = builder.build();
        let mut chain = Stack::new()?;
        for c in intermediates {
            chain.push(c.clone())?;
        }
        let mut ctx = X509StoreContext::new()?;
        ctx.init(&store, cert, &chain, |c| {
            let ok = c.verify_cert()?;
            Ok(if ok {
                Trust::Trusted
            } else if c.error().as_raw() == openssl_sys::X509_V_ERR_CERT_HAS_EXPIRED {
                Trust::Expired
            } else {
                Trust::Untrusted
            })
        })
    };
    check().unwrap_or(Trust::Untrusted)
}

/// O certificado vale agora (sem olhar a cadeia).
pub fn current(info: &CertInfo, now: i64) -> bool {
    info.not_before <= now && now < info.not_after
}

// ---------------------------------------------------------------- CMS (CA3)

mod ffi {
    // Presentes na libcrypto embutida; o crate `openssl-sys` não as declara.
    extern "C" {
        pub fn CMS_get0_signers(cms: *mut openssl_sys::CMS_ContentInfo) -> *mut openssl_sys::stack_st_X509;
        pub fn CMS_get1_certs(cms: *mut openssl_sys::CMS_ContentInfo) -> *mut openssl_sys::stack_st_X509;
    }
}

/// Quem assinou (só depois de um `verify` bem-sucedido).
fn signers(cms: &CmsContentInfoRef) -> Vec<X509> {
    // SAFETY: `CMS_get0_signers` devolve uma pilha nova cujos certificados pertencem ao CMS; cada um é
    // copiado (com referência própria) antes de a pilha ser liberada sem liberar os certificados.
    unsafe {
        let stack = ffi::CMS_get0_signers(cms.as_ptr());
        if stack.is_null() {
            return Vec::new();
        }
        let raw = stack as *mut openssl_sys::OPENSSL_STACK;
        let mut out = Vec::new();
        for i in 0..openssl_sys::OPENSSL_sk_num(raw) {
            let p = openssl_sys::OPENSSL_sk_value(raw, i) as *mut openssl_sys::X509;
            if !p.is_null() {
                out.push(X509Ref::from_ptr(p).to_owned());
            }
        }
        openssl_sys::OPENSSL_sk_free(raw);
        out
    }
}

/// Os certificados que vieram dentro da assinatura (o de quem assinou e os intermediários).
fn embedded_certs(cms: &CmsContentInfoRef) -> Vec<X509> {
    // SAFETY: `CMS_get1_certs` devolve uma pilha nova com uma referência a mais em cada certificado;
    // `Stack::from_ptr` passa a ser dona dela e libera tudo ao sair.
    unsafe {
        let stack = ffi::CMS_get1_certs(cms.as_ptr());
        if stack.is_null() {
            return Vec::new();
        }
        Stack::<X509>::from_ptr(stack).into_iter().collect()
    }
}

/// Cifra para os certificados dados (AES-256-CBC, o que Outlook e Apple Mail leem).
pub fn encrypt(content: &[u8], recipients: &[X509]) -> Result<Vec<u8>, AppError> {
    let mut stack = Stack::new().map_err(ssl_error("destinatários"))?;
    for r in recipients {
        stack.push(r.clone()).map_err(ssl_error("destinatários"))?;
    }
    let cms = CmsContentInfo::encrypt(&stack, content, Cipher::aes_256_cbc(), CMSOptions::BINARY).map_err(ssl_error("cifrar"))?;
    cms.to_der().map_err(ssl_error("cifrar"))
}

/// Assinatura destacada (SHA-256) dos bytes canônicos, com o meu certificado e a cadeia junto.
pub fn sign_detached(content: &[u8], cert: &X509Ref, key: &PKey<Private>, chain: &[X509]) -> Result<Vec<u8>, AppError> {
    let mut stack = Stack::new().map_err(ssl_error("cadeia"))?;
    for c in chain {
        stack.push(c.clone()).map_err(ssl_error("cadeia"))?;
    }
    let cms = CmsContentInfo::sign(Some(cert), Some(key), Some(&stack), Some(content), CMSOptions::DETACHED | CMSOptions::BINARY).map_err(ssl_error("assinar"))?;
    cms.to_der().map_err(ssl_error("assinar"))
}

/// Decifra com a primeira das minhas chaves que abrir.
pub fn decrypt(der: &[u8], mine: &[(X509, PKey<Private>)]) -> Result<Vec<u8>, AppError> {
    let cms = CmsContentInfo::from_der(der).map_err(ssl_error("ler CMS"))?;
    for (cert, key) in mine {
        if let Ok(out) = cms.decrypt(key, cert) {
            return Ok(out);
        }
    }
    Err(AppError::NoKeyForMessage)
}

/// Resultado de uma assinatura S/MIME: o conteúdo (o assinado, ou o que estava dentro), quem assinou e
/// os certificados que vieram junto.
pub enum Checked {
    Good { content: Vec<u8>, signer: X509, embedded: Vec<X509> },
    /// Nenhum certificado de quem assinou, nem na mensagem nem guardado.
    UnknownSigner,
    Bad,
}

/// Verifica a assinatura (destacada, com `detached`, ou opaca). Só a assinatura: a cadeia é `trust`.
pub fn verify(der: &[u8], detached: Option<&[u8]>, known: &[X509]) -> Checked {
    let Ok(mut cms) = CmsContentInfo::from_der(der) else { return Checked::Bad };
    let embedded = embedded_certs(&cms);
    let Ok(mut extra) = Stack::new() else { return Checked::Bad };
    for c in known {
        let _ = extra.push(c.clone());
    }
    let mut content = Vec::new();
    let flags = CMSOptions::NO_SIGNER_CERT_VERIFY | CMSOptions::BINARY;
    match cms.verify(Some(&extra), None, detached, Some(&mut content), flags) {
        Ok(()) => match signers(&cms).into_iter().next() {
            Some(signer) => Checked::Good { content: detached.map(<[u8]>::to_vec).unwrap_or(content), signer, embedded },
            None => Checked::Bad,
        },
        Err(_) if embedded.is_empty() && known.is_empty() => Checked::UnknownSigner,
        Err(_) => Checked::Bad,
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use openssl::bn::BigNum;
    use openssl::x509::extension::BasicConstraints;
    use openssl::x509::{X509Builder, X509Req};

    /// Uma CA de teste (raiz), como a do `scripts/smime-test-ca.ps1`.
    pub fn test_ca() -> (X509, PKey<Private>) {
        let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        let mut name = X509NameBuilder::new().unwrap();
        name.append_entry_by_nid(Nid::COMMONNAME, "CA de teste do Epistula").unwrap();
        name.append_entry_by_nid(Nid::ORGANIZATIONNAME, "Epistula Teste").unwrap();
        let name = name.build();
        let mut b = X509Builder::new().unwrap();
        b.set_version(2).unwrap();
        b.set_serial_number(&BigNum::from_u32(1).unwrap().to_asn1_integer().unwrap()).unwrap();
        b.set_subject_name(&name).unwrap();
        b.set_issuer_name(&name).unwrap();
        b.set_pubkey(&key).unwrap();
        b.set_not_before(&Asn1Time::days_from_now(0).unwrap()).unwrap();
        b.set_not_after(&Asn1Time::days_from_now(30).unwrap()).unwrap();
        b.append_extension(BasicConstraints::new().critical().ca().build().unwrap()).unwrap();
        b.append_extension(KeyUsage::new().critical().key_cert_sign().crl_sign().build().unwrap()).unwrap();
        b.sign(&key, MessageDigest::sha256()).unwrap();
        (b.build(), key)
    }

    /// A CA assina o pedido, copiando as extensões pedidas (como faz uma CA de e-mail).
    pub fn issue(csr_pem: &str, ca: &X509, ca_key: &PKey<Private>, days_ago: u32, days: u32) -> X509 {
        let req = X509Req::from_pem(csr_pem.as_bytes()).unwrap();
        let mut b = X509Builder::new().unwrap();
        b.set_version(2).unwrap();
        b.set_serial_number(&BigNum::from_u32(rand::random::<u32>() | 1).unwrap().to_asn1_integer().unwrap()).unwrap();
        b.set_subject_name(req.subject_name()).unwrap();
        b.set_issuer_name(ca.subject_name()).unwrap();
        b.set_pubkey(&req.public_key().unwrap()).unwrap();
        let start = Asn1Time::from_unix(crate::store::now() - i64::from(days_ago) * 86_400).unwrap();
        let end = Asn1Time::from_unix(crate::store::now() - i64::from(days_ago) * 86_400 + i64::from(days) * 86_400).unwrap();
        b.set_not_before(&start).unwrap();
        b.set_not_after(&end).unwrap();
        for ext in req.extensions().unwrap() {
            b.append_extension(ext).unwrap();
        }
        b.sign(ca_key, MessageDigest::sha256()).unwrap();
        b.build()
    }

    /// Pedido + emissão: o certificado e a chave de alguém.
    pub fn person(email: &str, kind: CertKind, ca: &X509, ca_key: &PKey<Private>) -> (X509, PKey<Private>) {
        let req = generate_request(kind, &Subject { name: "Pessoa", email, organization: Some("Lumen"), country: Some("br") }).unwrap();
        (issue(&req.csr_pem, ca, ca_key, 0, 365), private_key_from_der(&req.private_key).unwrap())
    }

    #[test]
    fn request_has_email_usage_and_matches_the_issued_certificate() {
        let (ca, ca_key) = test_ca();
        let req = generate_request(CertKind::Rsa3072, &Subject { name: "Yuri Melo", email: "Yuri@Exemplo.com", organization: None, country: Some("BR") }).unwrap();
        assert!(req.csr_pem.starts_with("-----BEGIN CERTIFICATE REQUEST-----"));
        let cert = issue(&req.csr_pem, &ca, &ca_key, 0, 365);
        assert_eq!(public_key_der(&cert).unwrap(), req.public_key, "o certificado casa com o pedido pela chave pública");
        let info = info(&cert);
        assert_eq!(info.subject, "Yuri Melo");
        assert_eq!(info.emails, vec!["yuri@exemplo.com".to_owned()]);
        assert_eq!(info.issuer, "Epistula Teste");
        assert_eq!(info.algorithm, "RSA 3072");
        assert!(info.can_sign && info.can_encrypt);
        assert!(!info.self_signed);
        assert_eq!(info.fingerprint.len(), 64);
    }

    #[test]
    fn trust_follows_the_roots_and_the_validity() {
        let (ca, ca_key) = test_ca();
        let (cert, _) = person("ana@x.com", CertKind::P256, &ca, &ca_key);
        assert_eq!(trust(&cert, &[], std::slice::from_ref(&ca), false), Trust::Trusted);
        assert_eq!(trust(&cert, &[], &[], false), Trust::Untrusted, "sem a raiz, não é confiável");
        let req = generate_request(CertKind::P256, &Subject { name: "Velho", email: "velho@x.com", organization: None, country: None }).unwrap();
        let old = issue(&req.csr_pem, &ca, &ca_key, 20, 10);
        assert_eq!(trust(&old, &[], std::slice::from_ref(&ca), false), Trust::Expired);
        assert_eq!(info(&cert).algorithm, "ECDSA P-256");
    }

    #[test]
    fn encrypt_and_sign_roundtrip_for_rsa_and_ec() {
        let (ca, ca_key) = test_ca();
        let (me, my_key) = person("eu@x.com", CertKind::Rsa3072, &ca, &ca_key);
        let (ana, ana_key) = person("ana@x.com", CertKind::P256, &ca, &ca_key);
        let content = b"Content-Type: text/plain\r\n\r\nOl\xc3\xa1, Ana.";
        let der = encrypt(content, &[ana.clone(), me.clone()]).unwrap();
        assert_eq!(decrypt(&der, &[(ana.clone(), ana_key)]).unwrap(), content);
        assert_eq!(decrypt(&der, &[(me.clone(), my_key.clone())]).unwrap(), content, "a cópia para mim abre");
        let (other, other_key) = person("outro@x.com", CertKind::P256, &ca, &ca_key);
        assert!(matches!(decrypt(&der, &[(other, other_key)]), Err(AppError::NoKeyForMessage)));

        let sig = sign_detached(content, &me, &my_key, std::slice::from_ref(&ca)).unwrap();
        let Checked::Good { signer, embedded, .. } = verify(&sig, Some(content), &[]) else { panic!("esperava assinatura boa") };
        assert_eq!(fingerprint(&signer), fingerprint(&me));
        assert_eq!(embedded.len(), 2, "o meu certificado e a CA vão junto");
        assert!(matches!(verify(&sig, Some(b"Content-Type: text/plain\r\n\r\nOutra coisa"), &[]), Checked::Bad));
    }

    #[test]
    fn pkcs12_roundtrip_requires_the_password() {
        let (ca, ca_key) = test_ca();
        let (me, key) = person("eu@x.com", CertKind::P256, &ca, &ca_key);
        let p12 = export_pkcs12("Eu", &key, &me, std::slice::from_ref(&ca), "senha longa do pfx").unwrap();
        assert!(is_pkcs12(&p12));
        assert!(matches!(parse_pkcs12(&p12, "errada"), Err(AppError::WrongPassword)));
        let bundle = parse_pkcs12(&p12, "senha longa do pfx").unwrap();
        assert_eq!(fingerprint(&bundle.cert), fingerprint(&me));
        assert_eq!(bundle.chain.len(), 1);
        assert!(matches!(export_pkcs12("Eu", &key, &me, &[], "curta"), Err(AppError::WeakPassword)));
    }

    #[test]
    fn certificates_are_read_from_pem_and_der() {
        let (ca, ca_key) = test_ca();
        let (me, _) = person("eu@x.com", CertKind::P256, &ca, &ca_key);
        let pem = format!("{}{}", cert_pem(&me).unwrap(), cert_pem(&ca).unwrap());
        assert_eq!(parse_certs(pem.as_bytes()).unwrap().len(), 2);
        assert_eq!(parse_certs(&cert_der(&me).unwrap()).unwrap().len(), 1);
        assert!(parse_certs(b"lixo").is_err());
        let chain = chain_from_pem(&chain_pem(&[me.clone(), ca.clone()]));
        assert_eq!(chain.len(), 2);
    }
}
