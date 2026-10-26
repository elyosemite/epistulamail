//! MIME do S/MIME (RFC 8551, US14): monta a parte cifrada (`application/pkcs7-mime`) e a assinada
//! (`multipart/signed` com `application/pkcs7-signature`) e, na leitura, encontra uma ou outra. A
//! criptografia em si fica em `smime.rs`. Nenhum conteúdo vai para log.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use lettre::message::header::{ContentTransferEncoding, ContentType};
use lettre::message::Body;
use mail_parser::{MessageParser, MimeHeaders, PartType};

use crate::pgp_mime::canonical;

/// Base64 em linhas de 76 caracteres, como o MIME pede.
fn base64_lines(der: &[u8]) -> String {
    let data = STANDARD.encode(der);
    data.as_bytes().chunks(76).map(|c| std::str::from_utf8(c).unwrap_or_default()).collect::<Vec<_>>().join("\r\n")
}

fn random_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Uma parte MIME montada à mão: o Content-Type e o corpo, exatamente como serão enviados.
pub struct Entity {
    pub content_type: String,
    pub body: Vec<u8>,
}

impl Entity {
    /// A parte inteira (cabeçalho e corpo), para ser cifrada.
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = format!("Content-Type: {}\r\n\r\n", self.content_type).into_bytes();
        out.extend_from_slice(&self.body);
        out
    }

    /// Cabeçalho e corpo para a lettre pôr no topo da mensagem.
    pub fn into_lettre(self) -> (ContentType, Body) {
        let ct = ContentType::parse(&self.content_type).unwrap_or_else(|_| ContentType::parse("application/octet-stream").expect("tipo fixo"));
        // Os bytes assinados não podem ser recodificados: a parte veio da lettre já codificada (ASCII)
        // e a assinatura vai em base64, então o corpo inteiro já é 7 bits.
        debug_assert!(self.body.is_ascii());
        (ct, Body::dangerous_pre_encoded(self.body, ContentTransferEncoding::SevenBit))
    }
}

/// `multipart/signed`: a parte assinada exatamente como foi assinada, e a assinatura em DER.
pub fn signed_entity(signed_part: &[u8], signature_der: &[u8]) -> Entity {
    let boundary = format!("epistula-{}", random_token());
    let mut raw = Vec::new();
    raw.extend_from_slice(format!("This is an S/MIME signed message\r\n--{boundary}\r\n").as_bytes());
    raw.extend_from_slice(signed_part);
    raw.extend_from_slice(
        format!(
            "\r\n--{boundary}\r\nContent-Type: application/pkcs7-signature; name=\"smime.p7s\"\r\nContent-Transfer-Encoding: base64\r\nContent-Disposition: attachment; filename=\"smime.p7s\"\r\nContent-Description: S/MIME digital signature\r\n\r\n"
        )
        .as_bytes(),
    );
    raw.extend_from_slice(base64_lines(signature_der).as_bytes());
    raw.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    Entity { content_type: format!("multipart/signed; protocol=\"application/pkcs7-signature\"; micalg=sha-256; boundary=\"{boundary}\""), body: raw }
}

/// `application/pkcs7-mime` com o envelope em base64 (o anexo `smime.p7m` que os programas reconhecem).
pub fn enveloped_entity(der: &[u8]) -> (ContentType, Body, [(&'static str, &'static str); 1]) {
    let ct = ContentType::parse("application/pkcs7-mime; smime-type=enveloped-data; name=\"smime.p7m\"").expect("tipo fixo");
    // A lettre codifica em base64 (com as quebras de linha).
    let body = Body::new_with_encoding(der.to_vec(), ContentTransferEncoding::Base64).unwrap_or_else(Body::new);
    (ct, body, [("Content-Disposition", "attachment; filename=\"smime.p7m\"")])
}

/// O que a leitura encontra numa mensagem com S/MIME.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// Envelope cifrado (DER).
    Enveloped(Vec<u8>),
    /// `multipart/signed`: os bytes assinados (canônicos), a parte assinada como veio e a assinatura (DER).
    Signed { content: Vec<u8>, part: Vec<u8>, signature: Vec<u8> },
    /// Assinatura opaca (`smime-type=signed-data`): o conteúdo vai dentro.
    Opaque(Vec<u8>),
    None,
}

fn is_pkcs7(ctype: &str, subtype: &str) -> bool {
    ctype == "application" && (subtype == "pkcs7-mime" || subtype == "x-pkcs7-mime")
}

/// Procura S/MIME no topo da mensagem (ou de uma parte decifrada).
pub fn find(raw: &[u8]) -> Found {
    let Some(message) = MessageParser::default().parse(raw) else { return Found::None };
    let Some(root) = message.parts.first() else { return Found::None };
    let Some(ct) = root.content_type() else { return Found::None };
    let ctype = ct.ctype().to_ascii_lowercase();
    let subtype = ct.subtype().unwrap_or_default().to_ascii_lowercase();
    if is_pkcs7(&ctype, &subtype) {
        let kind = ct.attribute("smime-type").unwrap_or_default().to_ascii_lowercase();
        let name = ct.attribute("name").or_else(|| root.attachment_name()).unwrap_or_default().to_ascii_lowercase();
        let der = root.contents().to_vec();
        return match kind.as_str() {
            "signed-data" => Found::Opaque(der),
            "enveloped-data" | "authenticated-enveloped-data" => Found::Enveloped(der),
            "" if name.ends_with(".p7m") => Found::Enveloped(der),
            _ => Found::None,
        };
    }
    let protocol = ct.attribute("protocol").unwrap_or_default().to_ascii_lowercase();
    if ctype == "multipart" && subtype == "signed" && (protocol == "application/pkcs7-signature" || protocol == "application/x-pkcs7-signature") {
        if let PartType::Multipart(children) = &root.body {
            if let [first, second, ..] = children.as_slice() {
                let signed = &message.parts[*first as usize];
                let sig = &message.parts[*second as usize];
                let (start, end) = (signed.offset_header as usize, signed.offset_end as usize);
                if start < end && end <= raw.len() {
                    let part = raw[start..end].to_vec();
                    return Found::Signed { content: canonical(&part), part, signature: sig.contents().to_vec() };
                }
            }
        }
    }
    Found::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smime::tests::{person, test_ca};
    use crate::smime::{self, CertKind, Checked};
    use lettre::message::SinglePart;

    #[test]
    fn signed_message_verifies_after_transport_and_detects_changes() {
        let (ca, ca_key) = test_ca();
        let (me, key) = person("eu@x.com", CertKind::Rsa3072, &ca, &ca_key);
        let part = canonical(&SinglePart::plain(String::from("Olá, Ana.\nLinha dois.")).formatted());
        let sig = smime::sign_detached(&part, &me, &key, &[]).unwrap();
        let (ct, body) = signed_entity(&part, &sig).into_lettre();
        let raw = lettre::Message::builder().from("eu@x.com".parse().unwrap()).to("ana@x.com".parse().unwrap()).subject("Oi").header(ct).body(body).unwrap().formatted();
        let Found::Signed { content, signature, .. } = find(&raw) else { panic!("esperava assinada") };
        assert_eq!(String::from_utf8_lossy(&content), String::from_utf8_lossy(&part));
        assert!(matches!(smime::verify(&signature, Some(&content), &[]), Checked::Good { .. }));
        let tampered = String::from_utf8_lossy(&raw).replace("Linha dois", "Linha 2").into_bytes();
        let Found::Signed { content, signature, .. } = find(&tampered) else { panic!("esperava assinada") };
        assert!(matches!(smime::verify(&signature, Some(&content), &[]), Checked::Bad));
    }

    #[test]
    fn enveloped_message_is_found_and_opens() {
        let (ca, ca_key) = test_ca();
        let (ana, ana_key) = person("ana@x.com", CertKind::P256, &ca, &ca_key);
        let inner = b"Content-Type: text/plain; charset=utf-8\r\n\r\nSegredo".to_vec();
        let der = smime::encrypt(&inner, std::slice::from_ref(&ana)).unwrap();
        let (ct, body, [(name, value)]) = enveloped_entity(&der);
        let header = lettre::message::header::HeaderValue::new(lettre::message::header::HeaderName::new_from_ascii_str(name), value.to_owned());
        let raw = lettre::Message::builder().from("eu@x.com".parse().unwrap()).to("ana@x.com".parse().unwrap()).subject("Oi").header(ct).raw_header(header).body(body).unwrap().formatted();
        assert!(!String::from_utf8_lossy(&raw).contains("Segredo"));
        let Found::Enveloped(found) = find(&raw) else { panic!("esperava cifrada") };
        assert_eq!(smime::decrypt(&found, &[(ana, ana_key)]).unwrap(), inner);
    }

    #[test]
    fn plain_and_openpgp_messages_are_not_smime() {
        assert_eq!(find(b"From: a@x.com\r\nSubject: s\r\n\r\nOi"), Found::None);
        let pgp = b"From: a@x.com\r\nContent-Type: multipart/signed; protocol=\"application/pgp-signature\"; boundary=b\r\n\r\n--b\r\n\r\nx\r\n--b\r\n\r\ny\r\n--b--\r\n";
        assert_eq!(find(pgp), Found::None);
    }
}
