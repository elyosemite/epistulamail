//! PGP/MIME (RFC 3156) e proteção do assunto (US13): monta o corpo cifrado ou assinado de uma
//! mensagem e, na leitura, encontra a parte cifrada ou a assinatura. A criptografia em si fica em
//! `openpgp.rs`. Nenhum conteúdo vai para log.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use lettre::message::header::{ContentTransferEncoding, ContentType, HeaderName, HeaderValue};
use lettre::message::{Body, MultiPart, SinglePart};
use mail_parser::{MessageParser, MimeHeaders, PartType};

use crate::error::AppError;

/// O assunto que vai no cabeçalho de fora quando o assunto é cifrado (como no Thunderbird).
pub const HIDDEN_SUBJECT: &str = "...";

/// Corpo da mensagem antes de cifrar ou assinar, como a lettre o monta.
pub enum Inner {
    Single(SinglePart),
    Multi(MultiPart),
}

impl Inner {
    pub fn formatted(&self) -> Vec<u8> {
        match self {
            Inner::Single(p) => p.formatted(),
            Inner::Multi(p) => p.formatted(),
        }
    }
}

/// `=?utf-8?b?...?=`, para um cabeçalho com texto qualquer.
fn encoded_word(text: &str) -> String {
    if text.is_ascii() && !text.contains(['\r', '\n']) {
        return text.to_owned();
    }
    format!("=?utf-8?b?{}?=", STANDARD.encode(text.as_bytes()))
}

/// Acrescenta o assunto verdadeiro à parte de dentro (cabeçalhos protegidos, `protected-headers="v1"`).
/// Só depois disso a parte é cifrada; quem não entende vê o assunto como uma linha a mais.
pub fn protect_subject(inner: &[u8], subject: &str) -> Vec<u8> {
    let text = String::from_utf8_lossy(inner);
    let (head, rest) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let mut lines: Vec<String> = Vec::new();
    for line in head.split("\r\n") {
        if !line.starts_with([' ', '\t']) && line.to_ascii_lowercase().starts_with("content-type:") && !line.contains("protected-headers") {
            lines.push(format!("{line}; protected-headers=\"v1\""));
        } else {
            lines.push(line.to_owned());
        }
    }
    let subject = encoded_word(subject.chars().filter(|c| !c.is_control()).collect::<String>().as_str());
    format!("Subject: {subject}\r\n{}\r\n\r\n{rest}", lines.join("\r\n")).into_bytes()
}

/// Corpo `multipart/encrypted` com o texto cifrado em armadura (RFC 3156, seção 4).
pub fn encrypted_body(armored: String) -> MultiPart {
    MultiPart::encrypted("application/pgp-encrypted".into())
        .singlepart(
            SinglePart::builder()
                .header(ContentType::parse("application/pgp-encrypted").expect("tipo fixo"))
                .header(ContentTransferEncoding::SevenBit)
                .body(String::from("Version: 1\r\n")),
        )
        .singlepart(
            SinglePart::builder()
                .header(ContentType::parse("application/octet-stream; name=\"encrypted.asc\"").expect("tipo fixo"))
                .header(ContentTransferEncoding::SevenBit)
                .body(armored),
        )
}

/// Corpo `multipart/signed`: a parte assinada exatamente como foi assinada, e a assinatura.
/// Montado à mão para os bytes enviados serem os mesmos que foram assinados.
pub fn signed_body(signed_part: &[u8], armored_sig: &str) -> (ContentType, Body) {
    let boundary = format!("epistula-{}", random_token());
    let mut raw = Vec::new();
    raw.extend_from_slice(format!("This is an OpenPGP/MIME signed message (RFC 4880 and 3156)\r\n--{boundary}\r\n").as_bytes());
    raw.extend_from_slice(signed_part);
    raw.extend_from_slice(format!("\r\n--{boundary}\r\nContent-Type: application/pgp-signature; name=\"signature.asc\"\r\nContent-Description: OpenPGP digital signature\r\nContent-Disposition: attachment; filename=\"signature.asc\"\r\n\r\n").as_bytes());
    raw.extend_from_slice(armored_sig.as_bytes());
    raw.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let content_type = ContentType::parse(&format!("multipart/signed; micalg=pgp-sha512; protocol=\"application/pgp-signature\"; boundary=\"{boundary}\""))
        .expect("tipo montado aqui");
    let body = Body::new_with_encoding(raw, lettre::message::header::ContentTransferEncoding::SevenBit).unwrap_or_else(Body::new);
    (content_type, body)
}

/// Os bytes que vão ser assinados: a parte em formato canônico (fim de linha CRLF, sem o CRLF final).
pub fn canonical(part: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(part).replace("\r\n", "\n").replace('\n', "\r\n");
    text.trim_end_matches("\r\n").as_bytes().to_vec()
}

fn random_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Valor do cabeçalho Autocrypt (Autocrypt 1.1): o endereço e a chave pública binária em base64, dobrada.
pub fn autocrypt_value(address: &str, public_key: &[u8]) -> String {
    let data = STANDARD.encode(public_key);
    let folded: Vec<&str> = data.as_bytes().chunks(76).map(|c| std::str::from_utf8(c).unwrap_or_default()).collect();
    format!("addr={}; prefer-encrypt=mutual; keydata=\r\n {}", address.trim().to_lowercase(), folded.join("\r\n "))
}

pub fn autocrypt_header(address: &str, public_key: &[u8]) -> HeaderValue {
    HeaderValue::new(HeaderName::new_from_ascii_str("Autocrypt"), autocrypt_value(address, public_key))
}

/// Chave de um cabeçalho Autocrypt recebido: só se o `addr` for o remetente.
pub fn autocrypt_key(header_value: &str, sender: &str) -> Option<Vec<u8>> {
    let mut addr = None;
    let mut keydata = None;
    for attr in header_value.split(';') {
        let (k, v) = attr.split_once('=')?;
        match k.trim().to_ascii_lowercase().as_str() {
            "addr" => addr = Some(v.trim().to_lowercase()),
            "keydata" => keydata = Some(v.chars().filter(|c| !c.is_whitespace()).collect::<String>()),
            _ => {}
        }
    }
    if addr? != sender.trim().to_lowercase() {
        return None;
    }
    STANDARD.decode(keydata?).ok()
}

/// O que a leitura encontra numa mensagem com OpenPGP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// `multipart/encrypted` (ou texto cifrado em linha): o texto cifrado.
    Encrypted(Vec<u8>),
    /// `multipart/signed`: os bytes assinados (a primeira parte) e a assinatura.
    Signed { content: Vec<u8>, signature: Vec<u8> },
    None,
}

fn content_type_of(part: &mail_parser::MessagePart<'_>) -> (String, String) {
    part.content_type()
        .map(|ct| (ct.ctype().to_ascii_lowercase(), ct.subtype().unwrap_or_default().to_ascii_lowercase()))
        .unwrap_or_default()
}

/// Procura OpenPGP no topo da mensagem (RFC 3156) ou num texto cifrado em linha.
pub fn find(raw: &[u8]) -> Found {
    let Some(message) = MessageParser::default().parse(raw) else { return Found::None };
    let Some(root) = message.parts.first() else { return Found::None };
    let (ctype, subtype) = content_type_of(root);
    let protocol = root.content_type().and_then(|ct| ct.attribute("protocol")).unwrap_or_default().to_ascii_lowercase();
    if ctype == "multipart" && subtype == "encrypted" && protocol == "application/pgp-encrypted" {
        if let PartType::Multipart(children) = &root.body {
            for &i in children {
                let part = &message.parts[i as usize];
                if content_type_of(part) == ("application".into(), "octet-stream".into()) {
                    return Found::Encrypted(part.contents().to_vec());
                }
            }
        }
        return Found::None;
    }
    if ctype == "multipart" && subtype == "signed" && protocol == "application/pgp-signature" {
        if let PartType::Multipart(children) = &root.body {
            if let [first, second, ..] = children.as_slice() {
                let signed = &message.parts[*first as usize];
                let sig = &message.parts[*second as usize];
                let start = signed.offset_header as usize;
                let end = signed.offset_end as usize;
                if start < end && end <= raw.len() {
                    return Found::Signed { content: canonical(&raw[start..end]), signature: sig.contents().to_vec() };
                }
            }
        }
        return Found::None;
    }
    // Texto cifrado em linha (PGP antigo, sem MIME): só o corpo de texto inteiro.
    if let Some(text) = message.body_text(0) {
        let trimmed = text.trim();
        if trimmed.starts_with("-----BEGIN PGP MESSAGE-----") && trimmed.ends_with("-----END PGP MESSAGE-----") {
            return Found::Encrypted(trimmed.as_bytes().to_vec());
        }
    }
    Found::None
}

/// Assunto protegido de uma parte decifrada, quando ela traz um.
pub fn protected_subject(inner: &[u8]) -> Option<String> {
    let message = MessageParser::default().parse_headers(inner)?;
    message.subject().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

/// A mensagem que a leitura mostra: os cabeçalhos de fora (De, Para, Data) com o corpo decifrado.
pub fn rebuild(outer: &[u8], inner: &[u8]) -> Result<Vec<u8>, AppError> {
    let text = String::from_utf8_lossy(outer);
    let head = text.split_once("\r\n\r\n").or_else(|| text.split_once("\n\n")).map(|(h, _)| h).ok_or(AppError::InvalidInput)?;
    let mut kept = Vec::new();
    let mut skipping = false;
    for line in head.lines() {
        let continuation = line.starts_with([' ', '\t']);
        if !continuation {
            let name = line.split(':').next().unwrap_or_default().trim().to_ascii_lowercase();
            skipping = name.starts_with("content-") || name == "mime-version";
        }
        if !skipping {
            kept.push(line);
        }
    }
    let mut out = kept.join("\r\n").into_bytes();
    out.extend_from_slice(b"\r\nMIME-Version: 1.0\r\n");
    out.extend_from_slice(inner);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openpgp::{self, KeyKind, Verification};

    fn inner() -> Inner {
        Inner::Single(SinglePart::plain(String::from("Olá, Ana.\nLinha dois.")))
    }

    #[test]
    fn subject_is_protected_inside_and_read_back() {
        let protected = protect_subject(&inner().formatted(), "Revisão do contrato");
        let text = String::from_utf8_lossy(&protected);
        assert!(text.starts_with("Subject: =?utf-8?b?"));
        assert!(text.contains("protected-headers=\"v1\""));
        assert_eq!(protected_subject(&protected).as_deref(), Some("Revisão do contrato"));
    }

    #[test]
    fn encrypted_message_roundtrips_through_mime() {
        let me = openpgp::generate(KeyKind::Curve25519, "Eu", "eu@x.com").unwrap();
        let ana = openpgp::generate(KeyKind::Curve25519, "Ana", "ana@x.com").unwrap();
        let body = protect_subject(&inner().formatted(), "Segredo");
        let armored = openpgp::encrypt(&body, &[ana.to_public_key(), me.to_public_key()], Some(&me)).unwrap();
        let message = lettre::Message::builder()
            .from("eu@x.com".parse().unwrap())
            .to("ana@x.com".parse().unwrap())
            .subject(HIDDEN_SUBJECT)
            .multipart(encrypted_body(armored))
            .unwrap();
        let raw = message.formatted();
        assert!(!String::from_utf8_lossy(&raw).contains("Segredo"));
        let Found::Encrypted(cipher) = find(&raw) else { panic!("esperava cifrada") };
        let out = openpgp::decrypt(&cipher, std::slice::from_ref(&ana), &[me.to_public_key()]).unwrap();
        assert!(matches!(out.verification, Verification::Good { .. }));
        assert_eq!(protected_subject(&out.data).as_deref(), Some("Segredo"));
        let shown = rebuild(&raw, &out.data).unwrap();
        let parsed = MessageParser::default().parse(&shown).unwrap();
        assert_eq!(parsed.body_text(0).map(|t| t.replace("\r\n", "\n")).as_deref().map(str::trim), Some("Olá, Ana.\nLinha dois."));
        assert_eq!(parsed.from().and_then(|f| f.first()).and_then(|a| a.address()), Some("eu@x.com"));
    }

    #[test]
    fn signed_message_verifies_after_transport_and_detects_changes() {
        let me = openpgp::generate(KeyKind::Curve25519, "Eu", "eu@x.com").unwrap();
        let part = inner().formatted();
        let signed = canonical(&part);
        let sig = openpgp::sign_detached(&signed, &me).unwrap();
        let (ct, body) = signed_body(&signed, &sig);
        let message = lettre::Message::builder().from("eu@x.com".parse().unwrap()).to("ana@x.com".parse().unwrap()).subject("Oi").header(ct).body(body).unwrap();
        let raw = message.formatted();
        let Found::Signed { content, signature } = find(&raw) else { panic!("esperava assinada") };
        assert!(matches!(openpgp::verify_detached(&content, &signature, &[me.to_public_key()]), Verification::Good { .. }));
        let tampered = String::from_utf8_lossy(&raw).replace("Linha dois", "Linha 2").into_bytes();
        let Found::Signed { content, signature } = find(&tampered) else { panic!("esperava assinada") };
        assert_eq!(openpgp::verify_detached(&content, &signature, &[me.to_public_key()]), Verification::Bad);
    }

    #[test]
    fn autocrypt_header_roundtrips_only_for_the_sender() {
        let me = openpgp::generate(KeyKind::Curve25519, "Eu", "eu@x.com").unwrap();
        let bytes = openpgp::public_bytes(&me.to_public_key()).unwrap();
        let value = autocrypt_value("Eu@X.com", &bytes);
        assert_eq!(autocrypt_key(&value, "eu@x.com"), Some(bytes.clone()));
        assert_eq!(autocrypt_key(&value, "outro@x.com"), None, "chave de outro endereço não vale");
    }

    #[test]
    fn plain_messages_have_nothing_to_find() {
        assert_eq!(find(b"From: a@x.com\r\nSubject: s\r\n\r\nOi"), Found::None);
    }
}
