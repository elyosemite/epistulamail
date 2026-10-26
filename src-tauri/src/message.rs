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
    pub followup: Option<crate::store::Followup>,
    /// Começo do texto, para a densidade Confortável (US7); ausente enquanto não foi buscado.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    pub has_files: bool,
    pub has_images: bool,
    /// Só na lista da pasta (US8): a conversa desta linha, com quantidade e participantes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread: Option<crate::store::ThreadSummary>,
    /// Só nos resultados de busca: a pasta onde a mensagem está.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_name: Option<String>,
    /// Só nos resultados de busca: id opaco dessa pasta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
}

/// Pasta de uma conta: id da conta + id da pasta no banco local. Viaja como texto opaco.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderKey {
    #[serde(rename = "a")]
    pub account: String,
    #[serde(rename = "f")]
    pub folder: i64,
}

/// Mensagem: pasta + UID. O UIDVALIDITY é conferido contra o da pasta no banco.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageKey {
    #[serde(rename = "a")]
    pub account: String,
    #[serde(rename = "f")]
    pub folder: i64,
    #[serde(rename = "u")]
    pub uid: u32,
}

/// Cursor de paginação: a próxima página tem as mensagens com UID menor que `before`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    #[serde(rename = "a")]
    pub account: String,
    #[serde(rename = "f")]
    pub folder: i64,
    #[serde(rename = "v")]
    pub uid_validity: u32,
    #[serde(rename = "b")]
    pub before: u32,
}

const MESSAGE_PREFIX: &str = "m2";
const CURSOR_PREFIX: &str = "c2";
const FOLDER_PREFIX: &str = "f2";

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

/// Ids de conta são UUIDs gerados pelo app (hex e hífens).
pub fn valid_account_id(id: &str) -> bool {
    id.len() == 36 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

fn checked<T>(value: T, account: &str) -> Result<T, AppError> {
    if valid_account_id(account) {
        Ok(value)
    } else {
        Err(AppError::InvalidId)
    }
}

impl FolderKey {
    pub fn encode(&self) -> String {
        encode(FOLDER_PREFIX, self)
    }
    pub fn decode(value: &str) -> Result<Self, AppError> {
        let key: Self = decode(FOLDER_PREFIX, value)?;
        let account = key.account.clone();
        checked(key, &account)
    }
}

impl MessageKey {
    pub fn encode(&self) -> String {
        encode(MESSAGE_PREFIX, self)
    }
    pub fn decode(value: &str) -> Result<Self, AppError> {
        let key: Self = decode(MESSAGE_PREFIX, value)?;
        let account = key.account.clone();
        checked(key, &account)
    }
}

impl Cursor {
    pub fn encode(&self) -> String {
        encode(CURSOR_PREFIX, self)
    }
    pub fn decode(value: &str) -> Result<Self, AppError> {
        let key: Self = decode(CURSOR_PREFIX, value)?;
        let account = key.account.clone();
        checked(key, &account)
    }
}

/// Campos de cabeçalho guardados no banco para a lista (sem corpo).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeaderFields {
    pub from_name: String,
    pub from_address: String,
    pub subject: String,
    pub date: Option<String>,
    pub to: Vec<Person>,
    pub cc: Vec<Person>,
    pub message_id: Option<String>,
    /// `In-Reply-To` e `References` (US8): ligam a mensagem à conversa.
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub has_attachments: bool,
    /// Lista de e-mails (US12): o `List-Id`, ou "-" quando só há `List-Unsubscribe`.
    pub list_id: Option<String>,
}

/// `List-Id: Nome <id.lista.exemplo>` vira `id.lista.exemplo`; sem `List-Id` mas com
/// `List-Unsubscribe`, "-" (é de uma lista, sem nome).
fn list_id_of(message: &mail_parser::Message<'_>) -> Option<String> {
    let raw = message.header_raw("List-Id").map(str::trim).filter(|v| !v.is_empty());
    if let Some(raw) = raw {
        let inner = match (raw.rfind('<'), raw.rfind('>')) {
            (Some(a), Some(b)) if a < b => &raw[a + 1..b],
            _ => raw,
        };
        let id = inner.trim().to_lowercase();
        if !id.is_empty() {
            return Some(id.chars().take(200).collect());
        }
    }
    message.header_raw("List-Unsubscribe").is_some().then(|| "-".to_owned())
}

/// Ids de mensagem sem `<>` e em minúsculas, para comparar.
pub fn normalize_id(id: &str) -> String {
    id.trim().trim_start_matches('<').trim_end_matches('>').trim().to_lowercase()
}

fn header_ids(value: &mail_parser::HeaderValue<'_>) -> Vec<String> {
    match value {
        mail_parser::HeaderValue::Text(t) => vec![normalize_id(t)],
        mail_parser::HeaderValue::TextList(list) => list.iter().map(|t| normalize_id(t)).collect(),
        _ => Vec::new(),
    }
    .into_iter()
    .filter(|id| !id.is_empty())
    .collect()
}

/// Raiz da conversa (US8): o primeiro id de `References`; sem ele, o `In-Reply-To`; sem os dois, a
/// própria mensagem. Funciona com qualquer provedor, sem extensão do servidor.
pub fn thread_root(fields: &HeaderFields) -> Option<String> {
    fields.references.first().cloned().or_else(|| fields.in_reply_to.clone()).or_else(|| fields.message_id.as_deref().map(normalize_id))
}

/// Decodifica os cabeçalhos da lista (inclusive encoded-words RFC 2047).
pub fn parse_header_fields(parser: &MessageParser, raw: &[u8]) -> Option<HeaderFields> {
    let message = parser.parse_headers(raw)?;
    let from = message.from().and_then(|a| a.first());
    let has_attachments = message.content_type().is_some_and(|ct| {
        ct.ctype().eq_ignore_ascii_case("multipart") && ct.subtype().is_some_and(|s| s.eq_ignore_ascii_case("mixed"))
    });
    Some(HeaderFields {
        from_name: from.and_then(|a| a.name()).unwrap_or_default().trim().to_owned(),
        from_address: from.and_then(|a| a.address()).unwrap_or_default().trim().to_owned(),
        subject: message.subject().unwrap_or_default().trim().to_owned(),
        date: message.date().map(|d| d.to_rfc3339()),
        to: people(message.to()),
        cc: people(message.cc()),
        message_id: message.message_id().map(|id| id.trim().to_owned()).filter(|id| !id.is_empty()),
        in_reply_to: header_ids(message.in_reply_to()).into_iter().next(),
        references: header_ids(message.references()),
        has_attachments,
        list_id: list_id_of(&message),
    })
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
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
    /// O HTML tem texto citado marcado e escondido (US8, D3). No texto puro, o frontend procura as linhas com `>`.
    pub has_quote: bool,
    /// Criptografia e assinatura (US13): preenchido por `e2ee::open`.
    pub security: Option<crate::e2ee::Security>,
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
            has_quote: false,
            security: None,
        };
    };

    let from = message.from().and_then(|a| a.first());
    let mut blocked_remote_images = 0;
    let mut has_quote = false;
    let body = match message.html_part(0) {
        Some(part) if part.is_text_html() => {
            let source = message.body_html(0).unwrap_or_default();
            let clean = sanitize::sanitize_email_html(&source, allow_remote_images);
            blocked_remote_images = clean.blocked_remote_images;
            has_quote = sanitize::has_quote(&clean.html);
            Body::Html { html: sanitize::email_document(&clean.html, allow_remote_images) }
        }
        _ => match message.body_text(0) {
            Some(text) if !text.trim().is_empty() => Body::Text { text: text.into_owned() },
            _ => Body::None,
        },
    };

    let attachments = attachment_infos(&message);

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
        has_quote,
        security: None,
    }
}

/// Os anexos de uma mensagem, sem montar a leitura (o download em segundo plano não lê o HTML, US9).
pub fn attachments_of(raw: &[u8]) -> Vec<AttachmentInfo> {
    MessageParser::default().parse(raw).map(|m| attachment_infos(&m)).unwrap_or_default()
}

fn attachment_infos(message: &mail_parser::Message<'_>) -> Vec<AttachmentInfo> {
    message
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
        .collect()
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

    const ACCOUNT: &str = "0f8b1c2d-3e4f-4a5b-8c6d-7e8f9a0b1c2d";

    #[test]
    fn opaque_ids_roundtrip_and_reject_tampering() {
        let k = MessageKey { account: ACCOUNT.into(), folder: 7, uid: 42 };
        let id = k.encode();
        assert!(id.starts_with("m2."));
        assert_eq!(MessageKey::decode(&id).unwrap(), k);
        assert_eq!(MessageKey::decode("m2.%%%"), Err(AppError::InvalidId));
        assert_eq!(MessageKey::decode(&id.replace("m2.", "c2.")), Err(AppError::InvalidId));
        assert_eq!(MessageKey::decode(&id[..id.len() - 3]), Err(AppError::InvalidId));
        assert_eq!(MessageKey::decode(&"m2.".repeat(2000)), Err(AppError::InvalidId));

        let c = Cursor { account: ACCOUNT.into(), folder: 1, uid_validity: 1, before: 900 };
        assert_eq!(Cursor::decode(&c.encode()).unwrap(), c);
        assert_eq!(Cursor::decode(&id), Err(AppError::InvalidId));

        let f = FolderKey { account: ACCOUNT.into(), folder: 3 };
        assert_eq!(FolderKey::decode(&f.encode()).unwrap(), f);
    }

    #[test]
    fn ids_with_bad_account_are_rejected() {
        for account in ["", "../x", "0f8b1c2d-3e4f-4a5b-8c6d-7e8f9a0b1c2dZ", "zzzzzzzz-zzzz-zzzz-zzzz-zzzzzzzzzzzz"] {
            let f = FolderKey { account: account.into(), folder: 1 }.encode();
            assert_eq!(FolderKey::decode(&f), Err(AppError::InvalidId), "{account}");
        }
    }

    #[test]
    fn decodes_encoded_word_headers() {
        let raw = b"From: =?UTF-8?Q?Jo=C3=A3o_Silva?= <joao@lumen.com.br>\r\nTo: Yuri <yuri@gmail.com>\r\nSubject: =?UTF-8?B?UmV2aXPDo28gZG8gY29udHJhdG8=?=\r\nDate: Thu, 23 Oct 2026 10:42:00 -0300\r\nMessage-ID: <abc@lumen>\r\nContent-Type: multipart/mixed; boundary=x\r\n\r\n";
        let h = parse_header_fields(&MessageParser::default(), raw).unwrap();
        assert_eq!(h.from_name, "João Silva");
        assert_eq!(h.from_address, "joao@lumen.com.br");
        assert_eq!(h.subject, "Revisão do contrato");
        assert!(h.date.as_deref().unwrap().starts_with("2026-10-23T10:42:00"));
        assert_eq!(h.to, vec![Person { name: "Yuri".into(), address: "yuri@gmail.com".into() }]);
        assert_eq!(h.message_id.as_deref(), Some("abc@lumen"));
        assert!(h.has_attachments);
        assert_eq!(thread_root(&h).as_deref(), Some("abc@lumen"), "sem resposta, a raiz é ela mesma");
    }

    #[test]
    fn mailing_lists_are_recognized() {
        let parse = |raw: &[u8]| parse_header_fields(&MessageParser::default(), raw).unwrap().list_id;
        assert_eq!(parse(b"List-Id: Ofertas da Loja <Ofertas.Loja.com.br>\r\nSubject: s\r\n\r\n").as_deref(), Some("ofertas.loja.com.br"));
        assert_eq!(parse(b"List-Unsubscribe: <mailto:sair@loja.com>\r\nSubject: s\r\n\r\n").as_deref(), Some("-"), "só o descadastro");
        assert_eq!(parse(b"Subject: s\r\n\r\n"), None);
    }

    #[test]
    fn replies_point_to_the_conversation_root() {
        let raw = b"Message-ID: <C@x>\r\nIn-Reply-To: <B@x>\r\nReferences: <A@X>\r\n <B@x>\r\nSubject: Re: s\r\n\r\n";
        let h = parse_header_fields(&MessageParser::default(), raw).unwrap();
        assert_eq!(h.in_reply_to.as_deref(), Some("b@x"));
        assert_eq!(h.references, vec!["a@x".to_owned(), "b@x".to_owned()]);
        assert_eq!(thread_root(&h).as_deref(), Some("a@x"));
        let only_parent = parse_header_fields(&MessageParser::default(), b"Message-ID: <C@x>\r\nIn-Reply-To: <B@x>\r\n\r\n").unwrap();
        assert_eq!(thread_root(&only_parent).as_deref(), Some("b@x"));
        assert_eq!(thread_root(&HeaderFields::default()), None);
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

    /// US2 CA8 e US3 CA8: leitura, banco e contas não podem registrar assunto, endereços, corpo,
    /// nome de anexo, nome de pasta, nome dado à conta nem o e-mail da conta.
    #[test]
    fn logs_never_contain_message_content() {
        let capture = global_log_capture();
        let v = view(HTML_MESSAGE, false);
        assert!(matches!(v.body, Body::Html { .. }));
        let _ = view(b"\xff\xfe lixo sem cabecalhos", false);
        let _ = parse_header_fields(&MessageParser::default(), HTML_MESSAGE);
        crate::store::tests::exercise_for_redaction();
        let logs = String::from_utf8_lossy(&capture.0.lock().unwrap()).into_owned();
        assert!(logs.contains("mensagem montada"), "o teste precisa observar algum log: {logs}");
        assert!(logs.contains("conta criada no banco"), "o teste precisa observar os logs do banco: {logs}");
        for secret in [
            "Revisão", "João", "joao@lumen.com.br", "yuri@gmail.com", "equipe@lumen", "segredo@oculto", "Olá", "Contrato.pdf", "tracker.example",
            "redacao.marcador", "NomeContaMarcador", "PastaMarcadora", "AssuntoMarcador", "CorpoMarcador", "AnexoMarcador", "remetente@exemplo", "FotoMarcador", "Rm90b01hcmNhZG9y",
        ] {
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
            let _ = MessageKey::decode(&s);
            let _ = Cursor::decode(&s);
            let _ = FolderKey::decode(&s);
        }
    }
}
