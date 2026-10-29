//! Outlook pessoal via Microsoft Graph (US20). O Gmail continua em IMAP. Aqui as pastas e mensagens
//! do Graph são gravadas no mesmo banco, com um UID local sintético e o id remoto ao lado.
//! Nenhum assunto, endereço, corpo ou anexo vai para log.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::json;

use crate::error::AppError;
use crate::mailbox::{RemoteFolder, Role};
use crate::message::{HeaderFields, MessageFlags, Person};
use crate::preview::{self, Preview};
use crate::provider::Provider;
use crate::session::{AccountHandle, AppState};
use crate::store::{self, ApplyReport, FolderRow, RemoteHeader, Snapshot};

const BASE: &str = "https://graph.microsoft.com/v1.0";
const PAGE: usize = 200;
const UID_BASE: u32 = 1_000_000_000;
const MAX_FOLDER_PAGES: usize = 4;
const GRAPH_TIMEOUT: Duration = Duration::from_secs(60);
const MESSAGE_SELECT: &str = "id,internetMessageId,conversationId,subject,from,toRecipients,ccRecipients,receivedDateTime,isRead,flag,hasAttachments,bodyPreview,size,parentFolderId";

/// A conta usa o backend Graph?
pub fn applies(handle: &AccountHandle) -> bool {
    matches!(handle.provider, Some(Provider::Microsoft))
}

#[derive(Deserialize)]
struct GraphErrorBody {
    error: GraphError,
}

#[derive(Deserialize)]
struct GraphError {
    code: Option<String>,
}

async fn request(
    state: &AppState,
    handle: &AccountHandle,
    method: reqwest::Method,
    url: &str,
    body: Option<serde_json::Value>,
    raw: Option<(&'static str, Vec<u8>)>,
) -> Result<reqwest::Response, AppError> {
    let mut force_refresh = false;
    for attempt in 0..2 {
        let token = state.access(handle, force_refresh).await?;
        let mut req = state.http.request(method.clone(), url).bearer_auth(token.as_str()).timeout(GRAPH_TIMEOUT);
        if let Some(body) = &body {
            req = req.json(body);
        }
        if let Some((content_type, bytes)) = &raw {
            req = req.header(reqwest::header::CONTENT_TYPE, *content_type).body(bytes.clone());
        }
        let response = req.send().await.map_err(|e| {
            tracing::warn!(error = %e.without_url(), "falha de rede no Microsoft Graph");
            AppError::ImapConnection
        })?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED && attempt == 0 {
            force_refresh = true;
            continue;
        }
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status();
        let code = response.json::<GraphErrorBody>().await.ok().and_then(|e| e.error.code).unwrap_or_default();
        tracing::warn!(%status, code, "Microsoft Graph recusou a requisição");
        return Err(match status {
            reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => AppError::ImapAuth,
            _ => AppError::ImapConnection,
        });
    }
    Err(AppError::ImapConnection)
}

async fn get_json<T: DeserializeOwned>(state: &AppState, handle: &AccountHandle, url: &str) -> Result<T, AppError> {
    let response = request(state, handle, reqwest::Method::GET, url, None, None).await?;
    response.json().await.map_err(|_| AppError::ImapConnection)
}

#[derive(Deserialize)]
struct GraphList<T> {
    value: Vec<T>,
    #[serde(rename = "@odata.nextLink")]
    next: Option<String>,
}

#[derive(Deserialize, Clone, Default)]
struct GraphFolder {
    #[serde(default)]
    id: String,
    #[serde(default, rename = "displayName")]
    display_name: String,
    #[serde(default, rename = "childFolderCount")]
    child_count: i64,
    #[serde(default, rename = "totalItemCount")]
    total: Option<i64>,
    #[serde(default, rename = "unreadItemCount")]
    unread: Option<i64>,
}

#[derive(Deserialize, Clone)]
struct EmailAddress {
    name: Option<String>,
    address: Option<String>,
}

#[derive(Deserialize, Clone)]
struct Recipient {
    #[serde(rename = "emailAddress")]
    email: Option<EmailAddress>,
}

#[derive(Deserialize, Clone)]
struct GraphFlag {
    #[serde(rename = "flagStatus")]
    status: Option<String>,
}

#[derive(Deserialize, Clone)]
struct GraphMessage {
    id: String,
    #[serde(rename = "internetMessageId")]
    message_id: Option<String>,
    #[serde(rename = "conversationId")]
    conversation_id: Option<String>,
    subject: Option<String>,
    from: Option<Recipient>,
    #[serde(rename = "toRecipients")]
    to: Option<Vec<Recipient>>,
    #[serde(rename = "ccRecipients")]
    cc: Option<Vec<Recipient>>,
    #[serde(rename = "receivedDateTime")]
    received: Option<String>,
    #[serde(rename = "isRead")]
    is_read: Option<bool>,
    flag: Option<GraphFlag>,
    #[serde(rename = "hasAttachments")]
    has_attachments: Option<bool>,
    #[serde(rename = "bodyPreview")]
    body_preview: Option<String>,
    size: Option<i64>,
}

struct Page {
    messages: Vec<(RemoteHeader, Preview)>,
    next: Option<String>,
    total: Option<u32>,
    unread: Option<u32>,
}

fn address(value: Option<&EmailAddress>) -> (String, String) {
    let Some(value) = value else { return (String::new(), String::new()) };
    (
        value.name.clone().unwrap_or_default().trim().to_owned(),
        value.address.clone().unwrap_or_default().trim().to_lowercase(),
    )
}

fn people(list: Option<&Vec<Recipient>>) -> Vec<Person> {
    list.into_iter()
        .flatten()
        .filter_map(|r| r.email.as_ref())
        .filter_map(|e| {
            let (name, address) = address(Some(e));
            (!address.is_empty()).then_some(Person { name, address })
        })
        .collect()
}

fn fields_of(message: &GraphMessage) -> HeaderFields {
    let (from_name, from_address) = address(message.from.as_ref().and_then(|r| r.email.as_ref()));
    HeaderFields {
        from_name,
        from_address,
        subject: message.subject.clone().unwrap_or_default().trim().to_owned(),
        date: message.received.clone(),
        to: people(message.to.as_ref()),
        cc: people(message.cc.as_ref()),
        message_id: Some(message.message_id.clone().unwrap_or_else(|| message.id.clone())),
        in_reply_to: None,
        // O Graph agrupa por conversationId; a raiz da conversa é esse id.
        references: message.conversation_id.clone().into_iter().collect(),
        has_attachments: message.has_attachments.unwrap_or(false),
        list_id: None,
    }
}

fn flags_of(message: &GraphMessage) -> MessageFlags {
    MessageFlags {
        seen: message.is_read.unwrap_or(false),
        flagged: message.flag.as_ref().and_then(|f| f.status.as_deref()) == Some("flagged"),
    }
}

fn preview_of(message: &GraphMessage) -> Preview {
    Preview {
        snippet: message.body_preview.clone().unwrap_or_default().trim().chars().take(preview::SNIPPET_CHARS).collect(),
        has_files: message.has_attachments.unwrap_or(false),
        has_images: false,
    }
}

fn message_url(folder: &FolderRow, top: usize) -> String {
    format!("{BASE}/me/mailFolders/{}/messages?$top={top}&$orderby=receivedDateTime desc&$select={MESSAGE_SELECT}", folder.imap_name)
}

async fn fetch_page(state: &AppState, handle: &AccountHandle, url: &str) -> Result<Page, AppError> {
    let list: GraphList<GraphMessage> = get_json(state, handle, url).await?;
    let messages = list
        .value
        .iter()
        .map(|m| {
            let header = RemoteHeader {
                uid: 0,
                remote_id: Some(m.id.clone()),
                fields: fields_of(m),
                internal_date: None,
                size: m.size.unwrap_or(0).clamp(0, u32::MAX as i64) as u32,
                flags: flags_of(m),
            };
            (header, preview_of(m))
        })
        .collect();
    Ok(Page { messages, next: list.next, total: None, unread: None })
}

async fn folder_counts(state: &AppState, handle: &AccountHandle, folder: &FolderRow) -> (Option<u32>, Option<u32>) {
    let url = format!("{BASE}/me/mailFolders/{}?$select=id,totalItemCount,unreadItemCount", folder.imap_name);
    match get_json::<GraphFolder>(state, handle, &url).await {
        Ok(f) => (
            f.total.and_then(|n| u32::try_from(n).ok()).or(folder.total),
            f.unread.and_then(|n| u32::try_from(n).ok()).or(folder.unread),
        ),
        Err(_) => (folder.total, folder.unread),
    }
}

async fn well_known(state: &AppState, handle: &AccountHandle) -> Result<HashMap<String, Role>, AppError> {
    let mut map = HashMap::new();
    for (name, role) in [
        ("inbox", Role::Inbox),
        ("sentitems", Role::Sent),
        ("drafts", Role::Drafts),
        ("deleteditems", Role::Trash),
        ("archive", Role::All),
        ("junkemail", Role::Spam),
    ] {
        let url = format!("{BASE}/me/mailFolders/{name}?$select=id");
        if let Ok(folder) = get_json::<GraphFolder>(state, handle, &url).await {
            map.insert(folder.id, role);
        }
    }
    Ok(map)
}

async fn skipped_folders(state: &AppState, handle: &AccountHandle) -> HashSet<String> {
    let mut skipped = HashSet::new();
    for name in [
        "outbox",
        "conflicts",
        "localfailures",
        "serverfailures",
        "syncissues",
        "recoverableitemsdeletions",
        "searchfolders",
        "scheduled",
        "msgfolderroot",
    ] {
        let url = format!("{BASE}/me/mailFolders/{name}?$select=id");
        if let Ok(folder) = get_json::<GraphFolder>(state, handle, &url).await {
            skipped.insert(folder.id);
        }
    }
    skipped
}

/// Pastas do Graph para o catálogo local, com os papéis conhecidos.
pub async fn folders(state: &AppState, handle: &AccountHandle) -> Result<Vec<RemoteFolder>, AppError> {
    let roles = well_known(state, handle).await?;
    let skipped = skipped_folders(state, handle).await;
    let mut queue = vec![format!(
        "{BASE}/me/mailFolders?$top=100&$select=id,displayName,childFolderCount,totalItemCount,unreadItemCount"
    )];
    let mut seen_urls = HashSet::new();
    let mut folders = Vec::new();
    let mut pages = 0;
    while let Some(url) = queue.pop() {
        if pages >= MAX_FOLDER_PAGES || !seen_urls.insert(url.clone()) {
            break;
        }
        pages += 1;
        let list: GraphList<GraphFolder> = get_json(state, handle, &url).await?;
        for folder in list.value {
            if folder.id.is_empty() || skipped.contains(&folder.id) || folder.display_name.trim().is_empty() {
                continue;
            }
            if folder.child_count > 0 {
                queue.push(format!(
                    "{BASE}/me/mailFolders/{}/childFolders?$top=100&$select=id,displayName,childFolderCount,totalItemCount,unreadItemCount",
                    folder.id
                ));
            }
            let role = roles.get(&folder.id).copied().unwrap_or(Role::Custom);
            folders.push(RemoteFolder {
                imap_name: folder.id,
                role,
                display_name: folder.display_name.trim().to_owned(),
                unread: folder.unread.and_then(|n| u32::try_from(n).ok()),
                total: folder.total.and_then(|n| u32::try_from(n).ok()),
            });
        }
        if let Some(next) = list.next {
            queue.push(next);
        }
    }
    Ok(folders)
}

fn apply_page(conn: &mut rusqlite::Connection, folder_id: i64, page: &Page, older: bool) -> Result<ApplyReport, AppError> {
    let folder = store::get_folder(conn, folder_id)?.ok_or(AppError::FolderNotFound)?;
    let reset = folder.uidvalidity != Some(1);
    let mut map = if reset { HashMap::new() } else { store::graph_uids(conn, folder_id)? };
    let mut headers = Vec::with_capacity(page.messages.len());
    let mut previews = Vec::with_capacity(page.messages.len());
    let mut oldest = folder.oldest_uid;
    let mut max_uid = map.values().copied().max().unwrap_or(0);
    if reset {
        let base = UID_BASE.saturating_sub(page.messages.len().saturating_sub(1) as u32);
        let mut uid = base;
        for (header, preview) in page.messages.iter().rev() {
            let mut header = header.clone();
            header.uid = uid;
            uid = uid.saturating_add(1);
            headers.push(header.clone());
            previews.push((header.uid, preview.clone()));
        }
        oldest = Some(base);
        max_uid = uid.saturating_sub(1);
    } else if older {
        let mut uid = oldest.unwrap_or(UID_BASE);
        for (header, preview) in page.messages.iter() {
            let mut header = header.clone();
            let remote = header.remote_id.clone().unwrap_or_default();
            if let Some(existing) = map.get(&remote) {
                header.uid = *existing;
            } else {
                if uid == 0 {
                    break;
                }
                uid -= 1;
                header.uid = uid;
                map.insert(remote, uid);
            }
            headers.push(header.clone());
            previews.push((header.uid, preview.clone()));
        }
        oldest = headers.iter().map(|h| h.uid).min().or(oldest);
    } else {
        let mut uid = max_uid.saturating_add(1);
        for (header, preview) in page.messages.iter().rev() {
            let mut header = header.clone();
            let remote = header.remote_id.clone().unwrap_or_default();
            if let Some(existing) = map.get(&remote) {
                header.uid = *existing;
            } else {
                header.uid = uid;
                uid = uid.saturating_add(1);
                map.insert(remote, header.uid);
            }
            headers.push(header.clone());
            previews.push((header.uid, preview.clone()));
        }
        max_uid = uid.saturating_sub(1);
        if oldest.is_none() {
            oldest = headers.iter().map(|h| h.uid).min();
        }
    }
    let uidnext = max_uid.checked_add(1);
    // Na sincronização periódica da primeira página, o link para as mensagens mais antigas já foi
    // consumido pelo backfill: não volte a apontá-lo para a segunda página.
    let complete = if reset || older { page.next.is_none() } else { folder.complete };
    let snapshot = Snapshot {
        reset,
        uidvalidity: 1,
        uidnext,
        highestmodseq: None,
        total: page.total.unwrap_or(0),
        unread: page.unread,
        headers,
        flag_updates: Vec::new(),
        present: None,
        oldest_uid: oldest,
        complete: Some(complete),
    };
    let report = store::apply_snapshot(conn, folder_id, &snapshot)?;
    store::save_previews(conn, folder_id, &previews)?;
    if reset || older {
        store::set_graph_next_link(conn, folder_id, page.next.as_deref())?;
    }
    Ok(report)
}

/// Sincroniza a primeira página (as mensagens mais recentes) de uma pasta.
pub async fn sync_folder(state: &AppState, handle: &AccountHandle, folder_id: i64) -> Result<ApplyReport, AppError> {
    let folder = state.db.call(move |c| store::get_folder(c, folder_id)).await?.ok_or(AppError::FolderNotFound)?;
    let (total, unread) = folder_counts(state, handle, &folder).await;
    let url = message_url(&folder, PAGE);
    let synced_from = store::now();
    let mut page = fetch_page(state, handle, &url).await?;
    page.total = total;
    page.unread = unread;
    state
        .db
        .call(move |c| {
            let report = apply_page(c, folder_id, &page, false)?;
            store::settle_moves(c, folder_id, synced_from)?;
            Ok(report)
        })
        .await
}

/// Busca a próxima página de mensagens mais antigas.
pub async fn fetch_older(state: &AppState, handle: &AccountHandle, folder: &FolderRow, _batch: u32) -> Result<usize, AppError> {
    let folder_id = folder.id;
    let Some(url) = state.db.call(move |c| store::graph_next_link(c, folder_id)).await? else { return Ok(0) };
    let mut page = fetch_page(state, handle, &url).await?;
    page.total = folder.total;
    page.unread = folder.unread;
    let folder_id = folder.id;
    let report = state.db.call(move |c| apply_page(c, folder_id, &page, true)).await?;
    Ok(report.upserted)
}

/// Corpo MIME completo da mensagem.
pub async fn body(state: &AppState, handle: &AccountHandle, folder_id: i64, uid: u32) -> Result<Vec<u8>, AppError> {
    let id = state.db.call(move |c| store::remote_id_for(c, folder_id, uid)).await?.ok_or(AppError::MessageNotFound)?;
    let url = format!("{BASE}/me/messages/{id}/$value");
    let response = request(state, handle, reqwest::Method::GET, &url, None, None).await?;
    response.bytes().await.map(|b| b.to_vec()).map_err(|_| AppError::ImapConnection)
}

/// Marca a mensagem como lida no Graph.
pub async fn mark_seen(state: &AppState, handle: &AccountHandle, folder_id: i64, uid: u32) -> Result<(), AppError> {
    let Some(id) = state.db.call(move |c| store::remote_id_for(c, folder_id, uid)).await? else { return Ok(()) };
    let url = format!("{BASE}/me/messages/{id}");
    request(state, handle, reqwest::Method::PATCH, &url, Some(json!({ "isRead": true })), None).await?;
    Ok(())
}

/// Altera lida/estrela no Graph e devolve as flags resultantes.
pub async fn set_flags(
    state: &AppState,
    handle: &AccountHandle,
    folder_id: i64,
    uid: u32,
    current: MessageFlags,
    seen: Option<bool>,
    flagged: Option<bool>,
) -> Result<MessageFlags, AppError> {
    let Some(id) = state.db.call(move |c| store::remote_id_for(c, folder_id, uid)).await? else { return Err(AppError::MessageNotFound) };
    let mut patch = serde_json::Map::new();
    if let Some(seen) = seen {
        patch.insert("isRead".into(), json!(seen));
    }
    if let Some(flagged) = flagged {
        patch.insert("flag".into(), json!({ "flagStatus": if flagged { "flagged" } else { "notFlagged" } }));
    }
    let url = format!("{BASE}/me/messages/{id}");
    request(state, handle, reqwest::Method::PATCH, &url, Some(serde_json::Value::Object(patch)), None).await?;
    Ok(MessageFlags { seen: seen.unwrap_or(current.seen), flagged: flagged.unwrap_or(current.flagged) })
}

async fn remote_ids(state: &AppState, folder_id: i64, uids: &[u32]) -> Result<Vec<String>, AppError> {
    let uids = uids.to_vec();
    state
        .db
        .call(move |c| {
            uids.iter()
                .map(|uid| store::remote_id_for(c, folder_id, *uid)?.ok_or(AppError::MessageNotFound))
                .collect()
        })
        .await
}

/// Move as mensagens para a pasta de destino do Graph.
pub async fn move_uids(state: &AppState, handle: &AccountHandle, source: &FolderRow, target: &FolderRow, uids: &[u32]) -> Result<(), AppError> {
    let ids = remote_ids(state, source.id, uids).await?;
    for id in ids {
        let url = format!("{BASE}/me/messages/{id}/move");
        request(state, handle, reqwest::Method::POST, &url, Some(json!({ "destinationId": target.imap_name })), None).await?;
    }
    Ok(())
}

/// Apaga de vez (já na Lixeira).
pub async fn delete_uids(state: &AppState, handle: &AccountHandle, folder: &FolderRow, uids: &[u32]) -> Result<(), AppError> {
    let ids = remote_ids(state, folder.id, uids).await?;
    for id in ids {
        let url = format!("{BASE}/me/messages/{id}");
        request(state, handle, reqwest::Method::DELETE, &url, None, None).await?;
    }
    Ok(())
}

/// Envia um MIME pronto pelo Graph: cria o rascunho, acrescenta o Cco fora dos cabeçalhos e envia.
pub async fn send(state: &AppState, handle: &AccountHandle, raw: Vec<u8>, bcc: &[String]) -> Result<(), AppError> {
    let create = format!("{BASE}/me/messages");
    let response = request(state, handle, reqwest::Method::POST, &create, None, Some(("text/rfc822", raw))).await.map_err(|e| match e {
        AppError::ImapConnection => AppError::SmtpConnection,
        other => other,
    })?;
    #[derive(Deserialize)]
    struct Created {
        id: String,
    }
    let created: Created = response.json().await.map_err(|_| AppError::SendRejected)?;
    // O MIME não leva Cco; o Graph recebe os destinatários ocultos no rascunho, fora dos cabeçalhos.
    if !bcc.is_empty() {
        let recipients: Vec<serde_json::Value> = bcc.iter().map(|address| json!({ "emailAddress": { "address": address } })).collect();
        let url = format!("{BASE}/me/messages/{}", created.id);
        request(state, handle, reqwest::Method::PATCH, &url, Some(json!({ "bccRecipients": recipients })), None)
            .await
            .map_err(|e| match e {
                AppError::ImapConnection => AppError::SmtpConnection,
                other => other,
            })?;
    }
    let send = format!("{BASE}/me/messages/{}/send", created.id);
    request(state, handle, reqwest::Method::POST, &send, None, None).await.map_err(|e| match e {
        AppError::ImapConnection => AppError::SmtpConnection,
        other => other,
    })?;
    Ok(())
}

/// Baixa os corpos MIME de uma lista de alvos (download em segundo plano).
pub async fn download(state: &AppState, handle: &AccountHandle, targets: &[store::BodyTarget]) -> Result<usize, AppError> {
    let mut saved = 0;
    for target in targets {
        let raw = match body(state, handle, target.folder_id, target.uid).await {
            Ok(raw) => raw,
            Err(AppError::MessageNotFound) => continue,
            Err(e) => return Err(e),
        };
        let attachments = crate::message::attachments_of(&raw);
        let preview = crate::preview::from_raw(&raw);
        let id = target.message_id;
        state.db.call(move |c| store::save_body_with(c, id, &raw, &attachments, &preview)).await?;
        saved += 1;
    }
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mailbox::Role;
    use crate::store::tests::{account, folder, header, test_db};

    fn graph_header(id: &str, conversation: &str) -> RemoteHeader {
        let mut header = header(0, id, "assunto", false);
        header.remote_id = Some(id.into());
        header.fields.references = vec![conversation.into()];
        header
    }

    fn page(ids: &[&str], next: Option<&str>) -> Page {
        Page {
            messages: ids.iter().map(|id| (graph_header(id, "conversa"), Preview::default())).collect(),
            next: next.map(str::to_owned),
            total: Some(ids.len() as u32),
            unread: Some(ids.len() as u32),
        }
    }

    #[test]
    fn graph_pages_keep_remote_ids_and_allocate_uids() {
        let t = test_db();
        t.db.with(|c| {
            let account = account(c, "outlook@exemplo.com");
            let folder = folder(c, &account.id, "graph-folder", Role::Inbox);
            let first = apply_page(c, folder.id, &page(&["g1", "g2"], Some("older")), false).unwrap();
            assert_eq!(first.upserted, 2);
            let uids = store::graph_uids(c, folder.id).unwrap();
            assert_eq!(uids.get("g1"), Some(&UID_BASE), "a mais nova da primeira página fica no topo");
            assert_eq!(uids.get("g2"), Some(&(UID_BASE - 1)));
            assert_eq!(store::remote_id_for(c, folder.id, UID_BASE).unwrap().as_deref(), Some("g1"));
            assert_eq!(store::graph_next_link(c, folder.id).unwrap().as_deref(), Some("older"));

            let older = apply_page(c, folder.id, &page(&["g0"], None), true).unwrap();
            assert_eq!(older.upserted, 1);
            let uids = store::graph_uids(c, folder.id).unwrap();
            assert_eq!(uids.get("g0"), Some(&(UID_BASE - 2)), "a página antiga fica abaixo da mais antiga");
            assert_eq!(store::graph_next_link(c, folder.id).unwrap(), None);
            Ok(())
        })
        .unwrap();
    }
}
