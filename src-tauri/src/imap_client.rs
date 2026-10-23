//! IMAP sobre TLS com SASL XOAUTH2: seleciona a INBOX e busca os cabeçalhos mais recentes.

use std::time::Duration;

use async_imap::types::Flag;
use futures::TryStreamExt;
use mail_parser::MessageParser;
use serde::Serialize;
use tokio::net::TcpStream;

use crate::error::AppError;
use crate::provider::Provider;

const IMAP_PORT: u16 = 993;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
pub const INBOX_PAGE: u32 = 50;

#[derive(Debug, Serialize, Clone, Copy, Default)]
pub struct MessageFlags {
    pub seen: bool,
    pub flagged: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageHeader {
    pub uid: u32,
    pub from_name: String,
    pub from_address: String,
    pub subject: String,
    pub date: String,
    pub flags: MessageFlags,
}

pub fn xoauth2_payload(user: &str, access_token: &str) -> String {
    format!("user={user}\x01auth=Bearer {access_token}\x01\x01")
}

/// Na primeira chamada envia as credenciais. Se o servidor responder com um desafio de erro,
/// a RFC do XOAUTH2 pede uma resposta vazia para encerrar a troca.
struct XOAuth2 {
    payload: String,
    sent: bool,
}

impl async_imap::Authenticator for XOAuth2 {
    type Response = String;

    fn process(&mut self, _challenge: &[u8]) -> String {
        if self.sent {
            return String::new();
        }
        self.sent = true;
        std::mem::take(&mut self.payload)
    }
}

/// Intervalo de sequência dos `count` mensagens mais recentes de uma pasta com `exists` mensagens.
pub fn latest_range(exists: u32, count: u32) -> Option<String> {
    if exists == 0 || count == 0 {
        return None;
    }
    let start = exists.saturating_sub(count - 1).max(1);
    Some(format!("{start}:{exists}"))
}

fn connection_error(context: &'static str) -> impl Fn(std::io::Error) -> AppError {
    move |e| {
        tracing::warn!(error = %e, context, "falha de conexão IMAP");
        AppError::ImapConnection
    }
}

fn imap_error(context: &'static str) -> impl Fn(async_imap::error::Error) -> AppError {
    move |e| {
        tracing::warn!(error = %e, context, "falha no comando IMAP");
        AppError::ImapConnection
    }
}

async fn with_timeout<T>(fut: impl std::future::Future<Output = Result<T, AppError>>, limit: Duration) -> Result<T, AppError> {
    tokio::time::timeout(limit, fut).await.map_err(|_| {
        tracing::warn!("operação IMAP excedeu o tempo limite");
        AppError::ImapConnection
    })?
}

type TlsStream = async_native_tls::TlsStream<TcpStream>;

async fn open_session(provider: Provider, email: &str, access_token: &str) -> Result<async_imap::Session<TlsStream>, AppError> {
    let host = provider.imap_host();
    let tls = with_timeout(
        async {
            let tcp = TcpStream::connect((host, IMAP_PORT)).await.map_err(connection_error("tcp"))?;
            async_native_tls::TlsConnector::new().connect(host, tcp).await.map_err(|e| {
                tracing::warn!(error = %e, "falha no handshake TLS");
                AppError::ImapConnection
            })
        },
        CONNECT_TIMEOUT,
    )
    .await?;

    with_timeout(
        async {
            let mut client = async_imap::Client::new(tls);
            client.read_response().await.ok_or(AppError::ImapConnection)?.map_err(connection_error("greeting"))?;
            let auth = XOAuth2 { payload: xoauth2_payload(email, access_token), sent: false };
            client.authenticate("XOAUTH2", auth).await.map_err(|(e, _)| match e {
                async_imap::error::Error::No(_) | async_imap::error::Error::Bad(_) => {
                    tracing::warn!(error = %e, ?provider, "servidor IMAP recusou o XOAUTH2");
                    AppError::ImapAuth
                }
                other => {
                    tracing::warn!(error = %other, ?provider, "falha durante o login IMAP");
                    AppError::ImapConnection
                }
            })
        },
        CONNECT_TIMEOUT,
    )
    .await
}

pub async fn fetch_inbox(provider: Provider, email: &str, access_token: &str) -> Result<Vec<MessageHeader>, AppError> {
    let mut session = open_session(provider, email, access_token).await?;

    with_timeout(
        async {
            let folders = session
                .list(Some(""), Some("*"))
                .await
                .map_err(imap_error("list"))?
                .try_fold(0usize, |n, _| async move { Ok(n + 1) })
                .await
                .map_err(imap_error("list"))?;
            tracing::debug!(folders, "pastas encontradas");

            let mailbox = session.select("INBOX").await.map_err(imap_error("select"))?;
            let mut headers = Vec::new();
            if let Some(range) = latest_range(mailbox.exists, INBOX_PAGE) {
                let fetches: Vec<_> = session
                    .fetch(range, "(UID FLAGS INTERNALDATE BODY.PEEK[HEADER.FIELDS (FROM SUBJECT DATE)])")
                    .await
                    .map_err(imap_error("fetch"))?
                    .try_collect()
                    .await
                    .map_err(imap_error("fetch"))?;
                let parser = MessageParser::default();
                headers = fetches.iter().filter_map(|f| to_header(&parser, f)).collect();
            }
            let _ = session.logout().await;

            headers.sort_by_key(|h| std::cmp::Reverse(h.uid));
            tracing::info!(count = headers.len(), "cabeçalhos da INBOX carregados");
            Ok(headers)
        },
        OPERATION_TIMEOUT,
    )
    .await
}

fn to_header(parser: &MessageParser, fetch: &async_imap::types::Fetch) -> Option<MessageHeader> {
    let uid = fetch.uid?;
    let mut flags = MessageFlags::default();
    for flag in fetch.flags() {
        match flag {
            Flag::Seen => flags.seen = true,
            Flag::Flagged => flags.flagged = true,
            _ => {}
        }
    }
    let internal_date = fetch.internal_date().map(|d| d.to_rfc3339());
    let parsed = fetch.header().and_then(|raw| parse_header_fields(parser, raw));
    let (from_name, from_address, subject, date) = parsed.unwrap_or_default();
    Some(MessageHeader {
        uid,
        from_name,
        from_address,
        subject,
        date: date.or(internal_date).unwrap_or_default(),
        flags,
    })
}

/// Decodifica From/Subject/Date (inclusive encoded-words RFC 2047).
pub fn parse_header_fields(parser: &MessageParser, raw: &[u8]) -> Option<(String, String, String, Option<String>)> {
    let message = parser.parse_headers(raw)?;
    let from = message.from().and_then(|a| a.first());
    let from_name = from.and_then(|a| a.name()).unwrap_or_default().trim().to_owned();
    let from_address = from.and_then(|a| a.address()).unwrap_or_default().trim().to_owned();
    let subject = message.subject().unwrap_or_default().trim().to_owned();
    let date = message.date().map(|d| d.to_rfc3339());
    Some((from_name, from_address, subject, date))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xoauth2_payload_follows_sasl_format() {
        assert_eq!(xoauth2_payload("a@b.com", "tok"), "user=a@b.com\x01auth=Bearer tok\x01\x01");
    }

    #[test]
    fn authenticator_sends_credentials_once() {
        use async_imap::Authenticator;
        let mut auth = XOAuth2 { payload: "p".into(), sent: false };
        assert_eq!(auth.process(b""), "p");
        assert_eq!(auth.process(b"{\"status\":\"400\"}"), "");
    }

    #[test]
    fn latest_range_takes_last_messages() {
        assert_eq!(latest_range(0, 50), None);
        assert_eq!(latest_range(10, 50), Some("1:10".into()));
        assert_eq!(latest_range(50, 50), Some("1:50".into()));
        assert_eq!(latest_range(1284, 50), Some("1235:1284".into()));
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
}
