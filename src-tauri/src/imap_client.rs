//! IMAP sobre TLS com SASL XOAUTH2. Uma sessão autenticada por conta é reaproveitada entre
//! comandos (`with_session`) e reaberta uma vez em caso de queda. Nada de conteúdo vai para log.

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use async_imap::types::{Fetch, Flag};
use futures::future::BoxFuture;
use futures::TryStreamExt;
use mail_parser::MessageParser;
use serde::Serialize;
use tokio::net::TcpStream;

use crate::error::AppError;
use crate::mailbox::{self, Folder, Role};
use crate::message::{self, Cursor, MessageFlags, MessageHeader, MessageRef, MessageView};
use crate::provider::Provider;
use crate::session::AppState;

const IMAP_PORT: u16 = 993;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
pub const PAGE_SIZE: u32 = 50;
pub const MAX_MESSAGE_BYTES: u32 = 25 * 1024 * 1024;
const HEADER_QUERY: &str = "(UID FLAGS INTERNALDATE BODY.PEEK[HEADER.FIELDS (FROM SUBJECT DATE)])";

type TlsStream = async_native_tls::TlsStream<TcpStream>;
pub type ImapSession = async_imap::Session<TlsStream>;

pub fn xoauth2_payload(user: &str, access_token: &str) -> String {
    format!("user={user}\x01auth=Bearer {access_token}\x01\x01")
}

/// Na primeira chamada envia as credenciais. Se o servidor responder com um desafio de erro,
/// o XOAUTH2 pede uma resposta vazia para encerrar a troca.
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

async fn open_session(provider: Provider, email: &str, access_token: &str) -> Result<ImapSession, AppError> {
    let host = provider.imap_host();
    with_timeout(
        async {
            let tcp = TcpStream::connect((host, IMAP_PORT)).await.map_err(connection_error("tcp"))?;
            let tls = async_native_tls::TlsConnector::new().connect(host, tcp).await.map_err(|e| {
                tracing::warn!(error = %e, "falha no handshake TLS");
                AppError::ImapConnection
            })?;
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

/// Executa `op` na sessão da conta, abrindo-a se preciso. Em queda de conexão (ou timeout),
/// descarta a sessão e tenta uma vez mais; se o login for recusado, força um refresh do token.
pub async fn with_session<T, F>(state: &AppState, mut op: F) -> Result<T, AppError>
where
    F: for<'a> FnMut(&'a mut ImapSession) -> BoxFuture<'a, Result<T, AppError>>,
{
    let mut guard = state.imap.lock().await;
    let mut force_refresh = false;
    for attempt in 0..2 {
        if guard.is_none() {
            let (account, token) = state.access(force_refresh).await?;
            match open_session(account.provider, &account.email, &token).await {
                Ok(session) => *guard = Some(session),
                Err(AppError::ImapAuth) if attempt == 0 => {
                    force_refresh = true;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        let Some(session) = guard.as_mut() else { return Err(AppError::ImapConnection) };
        match tokio::time::timeout(OPERATION_TIMEOUT, op(session)).await {
            Ok(Ok(value)) => return Ok(value),
            Ok(Err(AppError::ImapConnection)) | Err(_) if attempt == 0 => {
                tracing::info!("sessão IMAP descartada; reconectando");
                *guard = None;
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                *guard = None;
                return Err(AppError::ImapConnection);
            }
        }
    }
    Err(AppError::ImapConnection)
}

pub async fn close_session(state: &AppState) {
    if let Some(mut session) = state.imap.lock().await.take() {
        let _ = tokio::time::timeout(Duration::from_secs(3), session.logout()).await;
    }
}

fn flags_of(fetch: &Fetch) -> MessageFlags {
    let mut flags = MessageFlags::default();
    for flag in fetch.flags() {
        match flag {
            Flag::Seen => flags.seen = true,
            Flag::Flagged => flags.flagged = true,
            _ => {}
        }
    }
    flags
}

fn to_header(parser: &MessageParser, mailbox: &str, uid_validity: u32, fetch: &Fetch) -> Option<MessageHeader> {
    let uid = fetch.uid?;
    let internal_date = fetch.internal_date().map(|d| d.to_rfc3339());
    let (from_name, from_address, subject, date) = fetch
        .header()
        .and_then(|raw| message::parse_header_fields(parser, raw))
        .unwrap_or_default();
    Some(MessageHeader {
        id: MessageRef { mailbox: mailbox.to_owned(), uid_validity, uid }.encode(),
        from_name,
        from_address,
        subject,
        date: date.or(internal_date).unwrap_or_default(),
        flags: flags_of(fetch),
    })
}

/// Lista separada por vírgula para comandos `UID`. Os UIDs vêm de ids validados (números).
fn uid_set(uids: &[u32]) -> String {
    uids.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
}

/// Intervalo de sequência das `count` mensagens até `end` (inclusive).
pub fn page_range(end: u32, count: u32) -> Option<(u32, u32)> {
    if end == 0 || count == 0 {
        return None;
    }
    Some((end.saturating_sub(count - 1).max(1), end))
}

// ---------------------------------------------------------------- pastas

pub async fn list_folders(session: &mut ImapSession) -> Result<Vec<Folder>, AppError> {
    let names: Vec<_> = session
        .list(Some(""), Some("*"))
        .await
        .map_err(imap_error("list"))?
        .try_collect()
        .await
        .map_err(imap_error("list"))?;

    let mut folders: Vec<Folder> = names
        .iter()
        .filter_map(|n| {
            let role = mailbox::classify(n.name(), n.attributes())?;
            let name = role.label().map(str::to_owned).unwrap_or_else(|| mailbox::decode_mailbox_name(n.name()));
            Some(Folder { id: message::encode_folder_id(n.name()), role, name, unread: None, total: None, mailbox: n.name().to_owned() })
        })
        .collect();
    mailbox::sort_folders(&mut folders);

    for folder in folders.iter_mut().filter(|f| f.role.is_system()) {
        match session.status(&folder.mailbox, "(MESSAGES UNSEEN)").await {
            Ok(status) => {
                folder.total = Some(status.exists);
                folder.unread = status.unseen;
            }
            Err(e) => tracing::warn!(error = %e, role = ?folder.role, "STATUS falhou; contadores omitidos"),
        }
    }
    tracing::info!(
        folders = folders.len(),
        custom = folders.iter().filter(|f| f.role == Role::Custom).count(),
        "pastas carregadas"
    );
    Ok(folders)
}

// ---------------------------------------------------------------- lista paginada

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePage {
    pub items: Vec<MessageHeader>,
    pub next_cursor: Option<String>,
    pub total: u32,
    pub unread: Option<u32>,
}

async fn fetch_headers(session: &mut ImapSession, set: String, by_uid: bool) -> Result<Vec<Fetch>, AppError> {
    if by_uid {
        session.uid_fetch(set, HEADER_QUERY).await.map_err(imap_error("uid fetch"))?.try_collect().await.map_err(imap_error("uid fetch"))
    } else {
        session.fetch(set, HEADER_QUERY).await.map_err(imap_error("fetch"))?.try_collect().await.map_err(imap_error("fetch"))
    }
}

/// Número de sequência atual do UID `uid`, se a mensagem ainda existir.
async fn sequence_of(session: &mut ImapSession, uid: u32) -> Result<Option<u32>, AppError> {
    let fetches: Vec<Fetch> = session
        .uid_fetch(uid.to_string(), "(UID)")
        .await
        .map_err(imap_error("uid fetch"))?
        .try_collect()
        .await
        .map_err(imap_error("uid fetch"))?;
    Ok(fetches.iter().find(|f| f.uid == Some(uid)).map(|f| f.message))
}

pub async fn list_page(session: &mut ImapSession, mailbox: &str, cursor: Option<Cursor>) -> Result<MessagePage, AppError> {
    let unread = session.status(mailbox, "(UNSEEN)").await.ok().and_then(|s| s.unseen);
    let selected = session.select(mailbox).await.map_err(imap_error("select"))?;
    let uid_validity = selected.uid_validity.unwrap_or(0);
    let total = selected.exists;

    if let Some(c) = &cursor {
        if c.mailbox != mailbox || c.uid_validity != uid_validity {
            return Err(AppError::CursorExpired);
        }
    }
    let before = cursor.as_ref().map(|c| c.before);
    let parser = MessageParser::default();

    // Caminho normal: a âncora ainda existe, então a página são as `PAGE_SIZE` sequências anteriores.
    // Se a âncora sumiu, cai para `UID SEARCH`, que não depende de sequência.
    let end = match before {
        None => Some(total),
        Some(1) => Some(0),
        Some(b) => sequence_of(session, b).await?.map(|seq| seq - 1),
    };

    let (fetches, has_more) = match end {
        Some(end) => match page_range(end, PAGE_SIZE) {
            Some((start, end)) => (fetch_headers(session, format!("{start}:{end}"), false).await?, start > 1),
            None => (Vec::new(), false),
        },
        None => {
            let b = before.unwrap_or(1);
            let mut uids: Vec<u32> = session
                .uid_search(format!("UID 1:{}", b.saturating_sub(1).max(1)))
                .await
                .map_err(imap_error("uid search"))?
                .into_iter()
                .filter(|u| *u < b)
                .collect();
            uids.sort_unstable_by(|a, b| b.cmp(a));
            let more = uids.len() > PAGE_SIZE as usize;
            uids.truncate(PAGE_SIZE as usize);
            if uids.is_empty() {
                (Vec::new(), false)
            } else {
                (fetch_headers(session, uid_set(&uids), true).await?, more)
            }
        }
    };

    let in_page = |f: &&Fetch| before.is_none_or(|b| f.uid.is_some_and(|u| u < b));
    let mut page: Vec<(u32, MessageHeader)> = fetches
        .iter()
        .filter(in_page)
        .filter_map(|f| Some((f.uid?, to_header(&parser, mailbox, uid_validity, f)?)))
        .collect();
    page.sort_by_key(|(uid, _)| std::cmp::Reverse(*uid));

    let next_cursor = match (has_more, page.last()) {
        (true, Some((min, _))) => Some(Cursor { mailbox: mailbox.to_owned(), uid_validity, before: *min }.encode()),
        _ => None,
    };
    let items: Vec<MessageHeader> = page.into_iter().map(|(_, h)| h).collect();
    tracing::info!(items = items.len(), total, has_more = next_cursor.is_some(), "página carregada");
    Ok(MessagePage { items, next_cursor, total, unread })
}

// ---------------------------------------------------------------- leitura

async fn select_checked(session: &mut ImapSession, mailbox: &str, uid_validity: u32) -> Result<(), AppError> {
    let selected = session.select(mailbox).await.map_err(imap_error("select"))?;
    if selected.uid_validity != Some(uid_validity) {
        return Err(AppError::MessageNotFound);
    }
    Ok(())
}

pub async fn read_message(session: &mut ImapSession, r: MessageRef, allow_remote_images: bool) -> Result<MessageView, AppError> {
    select_checked(session, &r.mailbox, r.uid_validity).await?;

    let meta: Vec<Fetch> = session
        .uid_fetch(r.uid.to_string(), "(UID FLAGS RFC822.SIZE INTERNALDATE)")
        .await
        .map_err(imap_error("uid fetch"))?
        .try_collect()
        .await
        .map_err(imap_error("uid fetch"))?;
    let meta = meta.iter().find(|f| f.uid == Some(r.uid)).ok_or(AppError::MessageNotFound)?;
    if meta.size.unwrap_or(0) > MAX_MESSAGE_BYTES {
        tracing::info!(bytes = meta.size, "mensagem acima do limite");
        return Err(AppError::MessageTooLarge);
    }
    let mut flags = flags_of(meta);
    let internal_date = meta.internal_date().map(|d| d.to_rfc3339());

    let bodies: Vec<Fetch> = session
        .uid_fetch(r.uid.to_string(), "(UID BODY.PEEK[])")
        .await
        .map_err(imap_error("uid fetch"))?
        .try_collect()
        .await
        .map_err(imap_error("uid fetch"))?;
    let raw = bodies
        .iter()
        .find(|f| f.uid == Some(r.uid))
        .and_then(|f| f.body())
        .ok_or(AppError::MessageNotFound)?;
    if raw.len() > MAX_MESSAGE_BYTES as usize {
        return Err(AppError::MessageTooLarge);
    }

    let mut view = message::build_view(raw, r.encode(), flags, internal_date, allow_remote_images);

    // Abrir marca como lida, só depois de o corpo ter carregado.
    if !flags.seen {
        let _: Vec<Fetch> = session
            .uid_store(r.uid.to_string(), "+FLAGS.SILENT (\\Seen)")
            .await
            .map_err(imap_error("uid store"))?
            .try_collect()
            .await
            .map_err(imap_error("uid store"))?;
        flags.seen = true;
        view.flags = flags;
    }
    Ok(view)
}

// ---------------------------------------------------------------- flags

#[derive(Debug, Serialize, Clone)]
pub struct FlagResult {
    pub id: String,
    pub flags: MessageFlags,
}

/// Comandos `STORE` para as mudanças pedidas. `None` deixa a flag como está.
pub fn store_commands(seen: Option<bool>, flagged: Option<bool>) -> Vec<&'static str> {
    let mut commands = Vec::new();
    match seen {
        Some(true) => commands.push("+FLAGS.SILENT (\\Seen)"),
        Some(false) => commands.push("-FLAGS.SILENT (\\Seen)"),
        None => {}
    }
    match flagged {
        Some(true) => commands.push("+FLAGS.SILENT (\\Flagged)"),
        Some(false) => commands.push("-FLAGS.SILENT (\\Flagged)"),
        None => {}
    }
    commands
}

/// Aplica as flags e devolve o estado final lido do servidor. Idempotente: `+FLAGS`/`-FLAGS`
/// repetidos não mudam o resultado.
pub async fn set_flags(session: &mut ImapSession, refs: Vec<MessageRef>, seen: Option<bool>, flagged: Option<bool>) -> Result<Vec<FlagResult>, AppError> {
    let commands = store_commands(seen, flagged);
    let mut groups: BTreeMap<(String, u32), Vec<u32>> = BTreeMap::new();
    for r in &refs {
        groups.entry((r.mailbox.clone(), r.uid_validity)).or_default().push(r.uid);
    }

    let mut results = Vec::with_capacity(refs.len());
    for ((mailbox, uid_validity), uids) in groups {
        select_checked(session, &mailbox, uid_validity).await?;
        let set = uid_set(&uids);
        for command in &commands {
            let _: Vec<Fetch> = session
                .uid_store(&set, command)
                .await
                .map_err(imap_error("uid store"))?
                .try_collect()
                .await
                .map_err(imap_error("uid store"))?;
        }
        let current: Vec<Fetch> = session
            .uid_fetch(&set, "(UID FLAGS)")
            .await
            .map_err(imap_error("uid fetch"))?
            .try_collect()
            .await
            .map_err(imap_error("uid fetch"))?;
        let by_uid: HashMap<u32, MessageFlags> = current.iter().filter_map(|f| Some((f.uid?, flags_of(f)))).collect();
        for uid in uids {
            if let Some(flags) = by_uid.get(&uid) {
                results.push(FlagResult { id: MessageRef { mailbox: mailbox.clone(), uid_validity, uid }.encode(), flags: *flags });
            }
        }
    }
    if results.len() < refs.len() {
        tracing::info!(requested = refs.len(), updated = results.len(), "algumas mensagens não existem mais");
    }
    Ok(results)
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
    fn page_range_takes_previous_messages() {
        assert_eq!(page_range(0, 50), None);
        assert_eq!(page_range(10, 50), Some((1, 10)));
        assert_eq!(page_range(50, 50), Some((1, 50)));
        assert_eq!(page_range(1284, 50), Some((1235, 1284)));
        assert_eq!(page_range(1234, 50), Some((1185, 1234)));
    }

    #[test]
    fn store_commands_cover_each_change() {
        assert_eq!(store_commands(Some(true), None), ["+FLAGS.SILENT (\\Seen)"]);
        assert_eq!(store_commands(Some(false), Some(true)), ["-FLAGS.SILENT (\\Seen)", "+FLAGS.SILENT (\\Flagged)"]);
        assert!(store_commands(None, None).is_empty());
    }

    #[test]
    fn uid_set_is_comma_separated() {
        assert_eq!(uid_set(&[3, 10, 7]), "3,10,7");
    }
}
