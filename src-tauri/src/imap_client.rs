//! IMAP sobre TLS com SASL XOAUTH2. Cada conta tem até duas sessões autenticadas (interativa e de
//! sincronização), reaproveitadas entre comandos e reabertas uma vez em caso de queda.
//! Nada de conteúdo vai para log.

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use async_imap::imap_proto::SectionPath;
use async_imap::types::{Fetch, Flag, Mailbox};
use futures::future::BoxFuture;
use futures::TryStreamExt;
use mail_parser::MessageParser;
use tokio::net::TcpStream;

use crate::error::AppError;
use crate::mailbox::{self, RemoteFolder, Role};
use crate::message::{self, HeaderFields, MessageFlags};
use crate::preview::{self, Preview};
use crate::session::{AccountHandle, AppState, Lane};
use crate::store::RemoteHeader;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(60);
pub const MAX_MESSAGE_BYTES: u32 = 25 * 1024 * 1024;
const HEADER_QUERY: &str =
    "(UID FLAGS INTERNALDATE RFC822.SIZE BODY.PEEK[HEADER.FIELDS (FROM TO CC SUBJECT DATE MESSAGE-ID IN-REPLY-TO REFERENCES CONTENT-TYPE)])";
/// Só os cabeçalhos da conversa, para completar mensagens salvas antes da US8.
const THREAD_QUERY: &str = "(UID BODY.PEEK[HEADER.FIELDS (MESSAGE-ID IN-REPLY-TO REFERENCES)])";

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

async fn open_session(handle: &AccountHandle, access_token: &str) -> Result<ImapSession, AppError> {
    let host = handle.imap_host.as_str();
    let provider = handle.provider;
    with_timeout(
        async {
            let tcp = TcpStream::connect((host, handle.imap_port)).await.map_err(connection_error("tcp"))?;
            let tls = async_native_tls::TlsConnector::new().connect(host, tcp).await.map_err(|e| {
                tracing::warn!(error = %e, "falha no handshake TLS");
                AppError::ImapConnection
            })?;
            let mut client = async_imap::Client::new(tls);
            client.read_response().await.ok_or(AppError::ImapConnection)?.map_err(connection_error("greeting"))?;
            let auth = XOAuth2 { payload: xoauth2_payload(&handle.email, access_token), sent: false };
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

/// Executa `op` numa sessão da conta, abrindo-a se preciso. Em queda de conexão (ou timeout),
/// descarta a sessão e tenta uma vez mais; se o login for recusado, força um refresh do token.
pub async fn with_session<T, F>(state: &AppState, handle: &AccountHandle, lane: Lane, mut op: F) -> Result<T, AppError>
where
    F: for<'a> FnMut(&'a mut ImapSession) -> BoxFuture<'a, Result<T, AppError>>,
{
    let mut guard = handle.lane(lane).lock().await;
    let mut force_refresh = false;
    for attempt in 0..2 {
        if guard.is_none() {
            let token = state.access(handle, force_refresh).await?;
            match open_session(handle, &token).await {
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
                tracing::info!(?lane, "sessão IMAP descartada; reconectando");
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

fn to_remote_header(parser: &MessageParser, fetch: &Fetch) -> Option<RemoteHeader> {
    let uid = fetch.uid?;
    let fields: HeaderFields = fetch.header().and_then(|raw| message::parse_header_fields(parser, raw)).unwrap_or_default();
    Some(RemoteHeader {
        uid,
        fields,
        internal_date: fetch.internal_date().map(|d| d.to_rfc3339()),
        size: fetch.size.unwrap_or(0),
        flags: flags_of(fetch),
    })
}

/// Lista separada por vírgula para comandos `UID`. Os UIDs vêm do banco ou do servidor (números).
pub fn uid_set(uids: &[u32]) -> String {
    uids.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
}

/// Intervalo de sequência das `count` mensagens até `end` (inclusive).
pub fn page_range(end: u32, count: u32) -> Option<(u32, u32)> {
    if end == 0 || count == 0 {
        return None;
    }
    Some((end.saturating_sub(count - 1).max(1), end))
}

async fn collect(stream: impl futures::Stream<Item = async_imap::error::Result<Fetch>>, context: &'static str) -> Result<Vec<Fetch>, AppError> {
    stream.try_collect().await.map_err(imap_error(context))
}

// ---------------------------------------------------------------- pastas

pub async fn remote_folders(session: &mut ImapSession) -> Result<Vec<RemoteFolder>, AppError> {
    let names: Vec<_> = session
        .list(Some(""), Some("*"))
        .await
        .map_err(imap_error("list"))?
        .try_collect()
        .await
        .map_err(imap_error("list"))?;

    let mut folders: Vec<RemoteFolder> = names
        .iter()
        .filter_map(|n| {
            let role = mailbox::classify(n.name(), n.attributes())?;
            let display_name = role.label().map(str::to_owned).unwrap_or_else(|| mailbox::decode_mailbox_name(n.name()));
            Some(RemoteFolder { imap_name: n.name().to_owned(), role, display_name, unread: None, total: None })
        })
        .collect();
    mailbox::sort_folders(&mut folders);

    for folder in folders.iter_mut().filter(|f| f.role.is_system()) {
        match session.status(&folder.imap_name, "(MESSAGES UNSEEN)").await {
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
        "pastas listadas no servidor"
    );
    Ok(folders)
}

// ---------------------------------------------------------------- seleção e leitura de cabeçalhos

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selected {
    pub uidvalidity: u32,
    pub uidnext: Option<u32>,
    pub exists: u32,
    pub highestmodseq: Option<u64>,
}

fn selected(mb: &Mailbox) -> Selected {
    Selected { uidvalidity: mb.uid_validity.unwrap_or(0), uidnext: mb.uid_next, exists: mb.exists, highestmodseq: mb.highest_modseq }
}

/// `SELECT` com CONDSTORE (RFC 7162) quando o servidor aceita; senão, `SELECT` simples.
pub async fn select(session: &mut ImapSession, mailbox: &str) -> Result<Selected, AppError> {
    match session.select_condstore(mailbox).await {
        Ok(mb) => Ok(selected(&mb)),
        Err(async_imap::error::Error::Bad(_)) | Err(async_imap::error::Error::No(_)) => {
            let mb = session.select(mailbox).await.map_err(imap_error("select"))?;
            Ok(selected(&mb))
        }
        Err(e) => Err(imap_error("select")(e)),
    }
}

/// Seleciona a pasta e confere o UIDVALIDITY esperado: se mudou, os UIDs do banco não valem mais.
pub async fn select_checked(session: &mut ImapSession, mailbox: &str, uidvalidity: u32) -> Result<Selected, AppError> {
    let s = select(session, mailbox).await?;
    if s.uidvalidity != uidvalidity {
        return Err(AppError::MessageNotFound);
    }
    Ok(s)
}

pub async fn unseen_count(session: &mut ImapSession, mailbox: &str) -> Option<u32> {
    session.status(mailbox, "(UNSEEN)").await.ok().and_then(|s| s.unseen)
}

pub async fn headers_by_sequence(session: &mut ImapSession, start: u32, end: u32) -> Result<Vec<RemoteHeader>, AppError> {
    let fetches = collect(session.fetch(format!("{start}:{end}"), HEADER_QUERY).await.map_err(imap_error("fetch"))?, "fetch").await?;
    let parser = MessageParser::default();
    Ok(fetches.iter().filter_map(|f| to_remote_header(&parser, f)).collect())
}

pub async fn headers_by_uid(session: &mut ImapSession, set: &str) -> Result<Vec<RemoteHeader>, AppError> {
    let fetches = collect(session.uid_fetch(set, HEADER_QUERY).await.map_err(imap_error("uid fetch"))?, "uid fetch").await?;
    let parser = MessageParser::default();
    Ok(fetches.iter().filter_map(|f| to_remote_header(&parser, f)).collect())
}

/// Flags dos UIDs a partir de `from`; com `changed_since`, só as alteradas depois daquele modseq.
pub async fn flags_from(session: &mut ImapSession, from: u32, changed_since: Option<u64>) -> Result<Vec<(u32, MessageFlags)>, AppError> {
    let query = match changed_since {
        Some(modseq) => format!("(UID FLAGS) (CHANGEDSINCE {modseq})"),
        None => "(UID FLAGS)".to_owned(),
    };
    let fetches = collect(session.uid_fetch(format!("{from}:*"), query).await.map_err(imap_error("uid fetch"))?, "uid fetch").await?;
    Ok(fetches.iter().filter_map(|f| Some((f.uid?, flags_of(f)))).filter(|(uid, _)| *uid >= from).collect())
}

pub async fn search_uids(session: &mut ImapSession, from: u32, to: Option<u32>) -> Result<Vec<u32>, AppError> {
    let range = match to {
        Some(to) => format!("UID {from}:{to}"),
        None => format!("UID {from}:*"),
    };
    let mut uids: Vec<u32> = session
        .uid_search(range)
        .await
        .map_err(imap_error("uid search"))?
        .into_iter()
        .filter(|u| *u >= from && to.is_none_or(|t| *u <= t))
        .collect();
    uids.sort_unstable();
    Ok(uids)
}

/// Cabeçalhos da conversa dos UIDs (US8), para as mensagens salvas antes de a conversa existir.
pub async fn thread_headers(session: &mut ImapSession, uids: &[u32]) -> Result<Vec<(u32, HeaderFields)>, AppError> {
    if uids.is_empty() {
        return Ok(Vec::new());
    }
    let fetches = collect(session.uid_fetch(uid_set(uids), THREAD_QUERY).await.map_err(imap_error("uid fetch"))?, "uid fetch").await?;
    let parser = MessageParser::default();
    Ok(fetches
        .iter()
        .filter_map(|f| Some((f.uid?, f.header().and_then(|raw| message::parse_header_fields(&parser, raw)).unwrap_or_default())))
        .collect())
}

// ---------------------------------------------------------------- prévia da lista (US7, D2)

/// Prévia dos UIDs da pasta selecionada: `BODYSTRUCTURE` de todos e, por seção de texto, os
/// primeiros `PEEK_BYTES` (sem marcar como lida). UIDs que não existem mais ficam de fora.
pub async fn previews(session: &mut ImapSession, uids: &[u32]) -> Result<Vec<(u32, Preview)>, AppError> {
    if uids.is_empty() {
        return Ok(Vec::new());
    }
    let fetches = collect(session.uid_fetch(uid_set(uids), "(UID BODYSTRUCTURE)").await.map_err(imap_error("uid fetch"))?, "uid fetch").await?;
    let structures: Vec<(u32, preview::Structure)> = fetches.iter().filter_map(|f| Some((f.uid?, preview::analyze(f.bodystructure()?)))).collect();
    let mut by_section: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for (uid, s) in &structures {
        if let Some(text) = &s.text {
            by_section.entry(text.section.clone()).or_default().push(*uid);
        }
    }
    let mut texts: HashMap<u32, Vec<u8>> = HashMap::new();
    for (section, group) in by_section {
        let path = SectionPath::Part(section.split('.').filter_map(|n| n.parse().ok()).collect(), None);
        let query = format!("(UID BODY.PEEK[{section}]<0.{}>)", preview::PEEK_BYTES);
        let fetched = collect(session.uid_fetch(uid_set(&group), query).await.map_err(imap_error("uid fetch"))?, "uid fetch").await?;
        for f in &fetched {
            if let (Some(uid), Some(data)) = (f.uid, f.section(&path)) {
                texts.insert(uid, data.to_vec());
            }
        }
    }
    Ok(structures
        .into_iter()
        .map(|(uid, s)| {
            let snippet = s.text.as_ref().and_then(|t| texts.get(&uid).map(|b| preview::snippet_from_part(b, t))).unwrap_or_default();
            (uid, Preview { snippet, has_files: s.has_files, has_images: s.has_images })
        })
        .collect())
}

// ---------------------------------------------------------------- corpo e flags

pub struct RawMessage {
    pub raw: Vec<u8>,
    pub flags: MessageFlags,
    pub internal_date: Option<String>,
}

/// Mensagem completa por UID, com o limite de 25 MB conferido antes de baixar o corpo.
pub async fn fetch_raw(session: &mut ImapSession, uid: u32) -> Result<RawMessage, AppError> {
    let meta = collect(session.uid_fetch(uid.to_string(), "(UID FLAGS RFC822.SIZE INTERNALDATE)").await.map_err(imap_error("uid fetch"))?, "uid fetch").await?;
    let meta = meta.iter().find(|f| f.uid == Some(uid)).ok_or(AppError::MessageNotFound)?;
    if meta.size.unwrap_or(0) > MAX_MESSAGE_BYTES {
        tracing::info!(bytes = meta.size, "mensagem acima do limite");
        return Err(AppError::MessageTooLarge);
    }
    let flags = flags_of(meta);
    let internal_date = meta.internal_date().map(|d| d.to_rfc3339());
    let bodies = collect(session.uid_fetch(uid.to_string(), "(UID BODY.PEEK[])").await.map_err(imap_error("uid fetch"))?, "uid fetch").await?;
    let raw = bodies.iter().find(|f| f.uid == Some(uid)).and_then(|f| f.body()).ok_or(AppError::MessageNotFound)?;
    if raw.len() > MAX_MESSAGE_BYTES as usize {
        return Err(AppError::MessageTooLarge);
    }
    Ok(RawMessage { raw: raw.to_vec(), flags, internal_date })
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

/// Aplica as flags na pasta já selecionada e devolve o estado final lido do servidor.
/// Idempotente: `+FLAGS`/`-FLAGS` repetidos não mudam o resultado.
pub async fn store_flags(session: &mut ImapSession, uids: &[u32], seen: Option<bool>, flagged: Option<bool>) -> Result<HashMap<u32, MessageFlags>, AppError> {
    let set = uid_set(uids);
    for command in store_commands(seen, flagged) {
        let _ = collect(session.uid_store(&set, command).await.map_err(imap_error("uid store"))?, "uid store").await?;
    }
    let current = collect(session.uid_fetch(&set, "(UID FLAGS)").await.map_err(imap_error("uid fetch"))?, "uid fetch").await?;
    Ok(current.iter().filter_map(|f| Some((f.uid?, flags_of(f)))).collect())
}

// ---------------------------------------------------------------- mover, apagar, criar (US4)

async fn supports_move(session: &mut ImapSession) -> bool {
    session.capabilities().await.map(|c| c.has_str("MOVE")).unwrap_or(false)
}

/// Apaga de vez os UIDs da pasta selecionada (`\Deleted` + `UID EXPUNGE`, RFC 4315).
pub async fn expunge_uids(session: &mut ImapSession, uids: &[u32]) -> Result<(), AppError> {
    let set = uid_set(uids);
    let _ = collect(session.uid_store(&set, "+FLAGS.SILENT (\\Deleted)").await.map_err(imap_error("uid store"))?, "uid store").await?;
    let stream = session.uid_expunge(&set).await.map_err(imap_error("uid expunge"))?;
    let _: Vec<_> = stream.try_collect().await.map_err(imap_error("uid expunge"))?;
    Ok(())
}

/// Move os UIDs da pasta selecionada para `target` com `UID MOVE` (RFC 6851); sem suporte, copia
/// e apaga da origem.
pub async fn move_uids(session: &mut ImapSession, uids: &[u32], target: &str) -> Result<(), AppError> {
    let set = uid_set(uids);
    if supports_move(session).await {
        return session.uid_mv(&set, target).await.map_err(imap_error("uid move"));
    }
    session.uid_copy(&set, target).await.map_err(imap_error("uid copy"))?;
    expunge_uids(session, uids).await
}

/// Cria a pasta (nome já em UTF-7 modificado). `NO` do servidor costuma ser nome repetido.
pub async fn create_mailbox(session: &mut ImapSession, encoded: &str) -> Result<(), AppError> {
    session.create(encoded).await.map_err(|e| match e {
        async_imap::error::Error::No(_) => {
            tracing::info!("servidor recusou criar a pasta");
            AppError::FolderExists
        }
        other => imap_error("create")(other),
    })
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
        assert_eq!(page_range(5000, 2000), Some((3001, 5000)));
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
