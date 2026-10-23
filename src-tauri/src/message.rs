//! Mensagens: ids e cursores opacos, cabeçalhos da lista e a visão completa para leitura (US2).
//! Nenhuma função daqui registra assunto, endereço, corpo ou nome de anexo em log.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use mail_parser::{Address, MessageParser, MimeHeaders};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::sanitize;

const MAX_OPAQUE_LEN: usize = 4096;

#[derive(Debug, Serialize, Clone, Copy, Default, PartialEq, Eq)]
pub struct MessageFlags {
    pub seen: bool,
    pub flagged: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageHeader {
    pub id: String,
    pub from_name: String,
    pub from_address: String,
    pub subject: String,
    pub date: String,
    pub flags: MessageFlags,
}

/// Referência de uma mensagem: pasta IMAP + UIDVALIDITY + UID. Viaja como texto opaco.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRef {
    #[serde(rename = "m")]
    pub mailbox: String,
    #[serde(rename = "v")]
    pub uid_validity: u32,
    #[serde(rename = "u")]
    pub uid: u32,
}

/// Cursor de paginação: a próxima página tem as mensagens com UID menor que `before`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    #[serde(rename = "m")]
    pub mailbox: String,
    #[serde(rename = "v")]
    pub uid_validity: u32,
    #[serde(rename = "b")]
    pub before: u32,
}

const MESSAGE_PREFIX: &str = "m1";
const CURSOR_PREFIX: &str = "c1";
const FOLDER_PREFIX: &str = "f1";

fn encode<T: Serialize>(prefix: &str, value: &T) -> String {
    let json = serde_json::to_vec(value).unwrap_or_default();
    format!("{prefix}.{}", URL_SAFE_NO_PAD.encode(json))
}

fn decode<T: DeserializeOwned>(prefix: &str, value: &str) -> Result<T, AppError> {
    if value.len() > MAX_OPAQUE_LEN {
        return Err(AppError::InvalidId);
    }
    let payload = value
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix('.'))
        .ok_or(AppError::InvalidId)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).map_err(|_| AppError::InvalidId)?;
    serde_json::from_slice(&bytes).map_err(|_| AppError::InvalidId)
}

impl MessageRef {
    pub fn encode(&self) -> String {
        encode(MESSAGE_PREFIX, self)
    }
    pub fn decode(value: &str) -> Result<Self, AppError> {
        decode(MESSAGE_PREFIX, value)
    }
}

impl Cursor {
    pub fn encode(&self) -> String {
        encode(CURSOR_PREFIX, self)
    }
    pub fn decode(value: &str) -> Result<Self, AppError> {
        decode(CURSOR_PREFIX, value)
    }
}

pub fn encode_folder_id(mailbox: &str) -> String {
    encode(FOLDER_PREFIX, &mailbox)
}

pub fn decode_folder_id(value: &str) -> Result<String, AppError> {
    let name: String = decode(FOLDER_PREFIX, value)?;
    if name.is_empty() || name.contains(['\r', '\n', '\0']) {
        return Err(AppError::InvalidId);
    }
    Ok(name)
}

/// Decodifica From/Subject/Date dos cabeçalhos (inclusive encoded-words RFC 2047).
pub fn parse_header_fields(parser: &MessageParser, raw: &[u8]) -> Option<(String, String, String, Option<String>)> {
    let message = parser.parse_headers(raw)?;
    let from = message.from().and_then(|a| a.first());
    let from_name = from.and_then(|a| a.name()).unwrap_or_default().trim().to_owned();
    let from_address = from.and_then(|a| a.address()).unwrap_or_default().trim().to_owned();
    let subject = message.subject().unwrap_or_default().trim().to_owned();
    let date = message.date().map(|d| d.to_rfc3339());
    Some((from_name, from_address, subject, date))
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct Person {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInfo {
    pub name: String,
    pub mime_type: String,
    pub size: usize,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Body {
    Html { html: String },
    Text { text: String },
    None,
}

impl Body {
    fn kind(&self) -> &'static str {
        match self {
            Body::Html { .. } => "html",
            Body::Text { .. } => "text",
            Body::None => "none",
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageView {
    pub id: String,
    pub subject: String,
    pub from_name: String,
    pub from_address: String,
    pub to: Vec<Person>,
    pub cc: Vec<Person>,
    pub date: String,
    pub flags: MessageFlags,
    pub body: Body,
    pub blocked_remote_images: usize,
    pub attachments: Vec<AttachmentInfo>,
}

fn people(address: Option<&Address>) -> Vec<Person> {
    address
        .map(|a| {
            a.iter()
                .map(|addr| Person {
                    name: addr.name().unwrap_or_default().trim().to_owned(),
                    address: addr.address().unwrap_or_default().trim().to_owned(),
                })
                .filter(|p| !p.name.is_empty() || !p.address.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Monta a visão de leitura a partir da mensagem bruta. Nunca falha: uma mensagem que não
/// pode ser interpretada aparece sem corpo. `Bcc` nunca é lido.
pub fn build_view(raw: &[u8], id: String, flags: MessageFlags, internal_date: Option<String>, allow_remote_images: bool) -> MessageView {
    let parsed = MessageParser::default().parse(raw);
    let Some(message) = parsed else {
        tracing::warn!(bytes = raw.len(), "mensagem não pôde ser interpretada");
        return MessageView {
            id,
            subject: String::new(),
            from_name: String::new(),
            from_address: String::new(),
            to: Vec::new(),
            cc: Vec::new(),
            date: internal_date.unwrap_or_default(),
            flags,
            body: Body::None,
            blocked_remote_images: 0,
            attachments: Vec::new(),
        };
    };

    let from = message.from().and_then(|a| a.first());
    let mut blocked_remote_images = 0;
    let body = match message.html_part(0) {
        Some(part) if part.is_text_html() => {
            let source = message.body_html(0).unwrap_or_default();
            let clean = sanitize::sanitize_email_html(&source, allow_remote_images);
            blocked_remote_images = clean.blocked_remote_images;
            Body::Html { html: sanitize::email_document(&clean.html, allow_remote_images) }
        }
        _ => match message.body_text(0) {
            Some(text) if !text.trim().is_empty() => Body::Text { text: text.into_owned() },
            _ => Body::None,
        },
    };

    let attachments: Vec<AttachmentInfo> = message
        .attachments()
        .map(|part| AttachmentInfo {
            name: part.attachment_name().map(str::to_owned).unwrap_or_else(|| "(sem nome)".to_owned()),
            mime_type: part
                .content_type()
                .map(|ct| match ct.subtype() {
                    Some(sub) => format!("{}/{}", ct.ctype(), sub),
                    None => ct.ctype().to_owned(),
                })
                .unwrap_or_else(|| "application/octet-stream".to_owned())
                .to_ascii_lowercase(),
            size: part.len(),
        })
        .collect();

    tracing::debug!(
        body = body.kind(),
        attachments = attachments.len(),
        blocked_remote_images,
        bytes = raw.len(),
        "mensagem montada"
    );

    MessageView {
        id,
        subject: message.subject().unwrap_or_default().trim().to_owned(),
        from_name: from.and_then(|a| a.name()).unwrap_or_default().trim().to_owned(),
        from_address: from.and_then(|a| a.address()).unwrap_or_default().trim().to_owned(),
        to: people(message.to()),
        cc: people(message.cc()),
        date: message.date().map(|d| d.to_rfc3339()).or(internal_date).unwrap_or_default(),
        flags,
        body,
        blocked_remote_images,
        attachments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    const HTML_MESSAGE: &[u8] = b"From: =?UTF-8?Q?Jo=C3=A3o_Silva?= <joao@lumen.com.br>\r\n\
To: Yuri <yuri@gmail.com>, Rafael <rafael@empresa.com>\r\n\
Cc: equipe@lumen.com.br\r\n\
Bcc: segredo@oculto.com\r\n\
Subject: =?UTF-8?B?UmV2aXPDo28gZG8gY29udHJhdG8=?=\r\n\
Date: Thu, 23 Oct 2026 10:42:00 -0300\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"b1\"\r\n\r\n\
--b1\r\n\
Content-Type: multipart/alternative; boundary=\"b2\"\r\n\r\n\
--b2\r\n\
Content-Type: text/plain; charset=utf-8\r\n\r\n\
Versao texto\r\n\
--b2\r\n\
Content-Type: text/html; charset=utf-8\r\n\r\n\
<p>Ol\xc3\xa1</p><img src=\"https://tracker.example/p.gif\"><script>alert(1)</script>\r\n\
--b2--\r\n\
--b1\r\n\
Content-Type: application/pdf; name=\"Contrato.pdf\"\r\n\
Content-Disposition: attachment; filename=\"Contrato.pdf\"\r\n\
Content-Transfer-Encoding: base64\r\n\r\n\
JVBERi0xLjQK\r\n\
--b1--\r\n";

    const TEXT_MESSAGE: &[u8] = b"From: Ana <ana@gmail.com>\r\nTo: yuri@gmail.com\r\nSubject: Oi\r\nDate: Thu, 23 Oct 2026 08:00:00 +0000\r\n\r\nPrimeiro paragrafo.\r\n\r\nSegundo.\r\n";

    fn view(raw: &[u8], allow: bool) -> MessageView {
        build_view(raw, "m1.x".into(), MessageFlags::default(), None, allow)
    }

    #[test]
    fn opaque_ids_roundtrip_and_reject_tampering() {
        let r = MessageRef { mailbox: "[Gmail]/Enviados".into(), uid_validity: 7, uid: 42 };
        let id = r.encode();
        assert!(id.starts_with("m1."));
        assert_eq!(MessageRef::decode(&id).unwrap(), r);
        assert_eq!(MessageRef::decode("m1.%%%"), Err(AppError::InvalidId));
        assert_eq!(MessageRef::decode(&id.replace("m1.", "c1.")), Err(AppError::InvalidId));
        assert_eq!(MessageRef::decode(&id[..id.len() - 3]), Err(AppError::InvalidId));
        assert_eq!(MessageRef::decode(&"m1.".repeat(2000)), Err(AppError::InvalidId));

        let c = Cursor { mailbox: "INBOX".into(), uid_validity: 1, before: 900 };
        assert_eq!(Cursor::decode(&c.encode()).unwrap(), c);
        assert_eq!(Cursor::decode(&id), Err(AppError::InvalidId));
    }

    #[test]
    fn folder_ids_roundtrip_and_reject_control_chars() {
        let id = encode_folder_id("Projetos/2026");
        assert_eq!(decode_folder_id(&id).unwrap(), "Projetos/2026");
        assert_eq!(decode_folder_id(&encode_folder_id("a\r\nLOGOUT")), Err(AppError::InvalidId));
        assert_eq!(decode_folder_id(&encode_folder_id("")), Err(AppError::InvalidId));
    }

    #[test]
    fn decodes_encoded_word_headers() {
        let raw = b"From: =?UTF-8?Q?Jo=C3=A3o_Silva?= <joao@lumen.com.br>\r\nSubject: =?UTF-8?B?UmV2aXPDo28gZG8gY29udHJhdG8=?=\r\nDate: Thu, 23 Oct 2026 10:42:00 -0300\r\n\r\n";
        let (name, addr, subject, date) = parse_header_fields(&MessageParser::default(), raw).unwrap();
        assert_eq!(name, "João Silva");
        assert_eq!(addr, "joao@lumen.com.br");
        assert_eq!(subject, "Revisão do contrato");
        assert!(date.unwrap().starts_with("2026-10-23T10:42:00"));
    }

    #[test]
    fn html_message_is_sanitized_with_recipients_and_attachments() {
        let v = view(HTML_MESSAGE, false);
        assert_eq!(v.subject, "Revisão do contrato");
        assert_eq!(v.from_name, "João Silva");
        assert_eq!(v.to.len(), 2);
        assert_eq!(v.cc, vec![Person { name: String::new(), address: "equipe@lumen.com.br".into() }]);
        let Body::Html { html } = &v.body else { panic!("esperava HTML") };
        assert!(html.contains("Olá"));
        assert!(!html.contains("<script"));
        assert!(!html.contains("tracker.example"));
        assert_eq!(v.blocked_remote_images, 1);
        assert_eq!(v.attachments, vec![AttachmentInfo { name: "Contrato.pdf".into(), mime_type: "application/pdf".into(), size: 9 }]);
        let json = serde_json::to_string(&v).unwrap();
        assert!(!json.contains("segredo@oculto.com"), "Bcc não pode sair do backend");
    }

    #[test]
    fn remote_images_are_kept_with_consent() {
        let Body::Html { html } = view(HTML_MESSAGE, true).body else { panic!() };
        assert!(html.contains("https://tracker.example/p.gif"));
    }

    #[test]
    fn text_message_keeps_plain_body() {
        let v = view(TEXT_MESSAGE, false);
        assert_eq!(v.body, Body::Text { text: "Primeiro paragrafo.\r\n\r\nSegundo.\r\n".into() });
        assert!(v.attachments.is_empty());
    }

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);
    impl Write for Capture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Subscriber global do binário de teste: um subscriber com escopo perde logs quando outros
    /// testes em paralelo registram os mesmos pontos de log antes dele. Global, ele também
    /// captura os logs da suíte inteira, e todos passam pela mesma verificação de vazamento.
    fn global_log_capture() -> &'static Capture {
        static CAPTURE: std::sync::OnceLock<Capture> = std::sync::OnceLock::new();
        CAPTURE.get_or_init(|| {
            let capture = Capture::default();
            let writer = capture.clone();
            let subscriber = tracing_subscriber::fmt()
                .with_max_level(tracing::Level::TRACE)
                .with_writer(move || writer.clone())
                .finish();
            tracing::subscriber::set_global_default(subscriber).expect("subscriber global de teste");
            capture
        })
    }

    /// CA8: o pipeline de leitura não pode registrar assunto, endereços, corpo nem nome de anexo.
    #[test]
    fn logs_never_contain_message_content() {
        let capture = global_log_capture();
        let v = view(HTML_MESSAGE, false);
        assert!(matches!(v.body, Body::Html { .. }));
        let _ = view(b"\xff\xfe lixo sem cabecalhos", false);
        let _ = parse_header_fields(&MessageParser::default(), HTML_MESSAGE);
        let logs = String::from_utf8_lossy(&capture.0.lock().unwrap()).into_owned();
        assert!(logs.contains("mensagem montada"), "o teste precisa observar algum log: {logs}");
        for secret in ["Revisão", "João", "joao@lumen.com.br", "yuri@gmail.com", "equipe@lumen", "segredo@oculto", "Olá", "Contrato.pdf", "tracker.example"] {
            assert!(!logs.contains(secret), "log vazou {secret:?}: {logs}");
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn arbitrary_bytes_never_panic(raw in proptest::collection::vec(any::<u8>(), 0..2048)) {
            let _ = view(&raw, false);
            let _ = parse_header_fields(&MessageParser::default(), &raw);
        }

        #[test]
        fn arbitrary_ids_never_panic(s in ".{0,300}") {
            let _ = MessageRef::decode(&s);
            let _ = Cursor::decode(&s);
            let _ = decode_folder_id(&s);
        }
    }
}
