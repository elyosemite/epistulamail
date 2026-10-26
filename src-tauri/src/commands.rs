use std::collections::BTreeMap;

use futures::FutureExt;
use reqwest::Url;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::AppError;
use crate::imap_client;
use crate::mailbox::Folder;
use crate::message::{self, Cursor, FolderKey, MessageFlags, MessageHeader, MessageKey, MessageView};
use crate::oauth;
use crate::providers::{self, AuthKind, ProviderView};
use crate::session::{AppState, Connected, Lane};
use crate::compose::{self, Template};
use crate::mailbox::{self, Role};
use crate::store::{self, Account, AccountStatus, Draft, DraftContent, DraftKind, FolderRow, Followup, FollowupChange};
use crate::links;
use crate::{openpgp, pgp_store};
use crate::{rule_runner, rules};
use crate::privacy::{self, LockStatus};
use crate::{moves, send, sync};
use zeroize::Zeroizing;

const MAX_FLAG_IDS: usize = 500;
const MAX_URL_LEN: usize = 2048;
pub const PAGE_SIZE: u32 = 50;

fn account_id(id: &str) -> Result<String, AppError> {
    if message::valid_account_id(id) {
        Ok(id.to_owned())
    } else {
        Err(AppError::InvalidId)
    }
}

/// Bloqueia ações que precisam do servidor quando o usuário saiu da conta ou ela perdeu o acesso.
fn require_active(account: &Account) -> Result<(), AppError> {
    match account.status {
        AccountStatus::Active => Ok(()),
        AccountStatus::SignedOut => Err(AppError::SignedOut),
        AccountStatus::NeedsReauth => Err(AppError::TokenExpired),
    }
}

// ---------------------------------------------------------------- contas

#[tauri::command]
pub async fn list_accounts(state: State<'_, AppState>) -> Result<Vec<Account>, AppError> {
    state.wait_started().await;
    state.db.call(|c| store::list_accounts(c)).await
}

/// O frontend mostrou a primeira tela com as contas: libera a sincronização automática, que até
/// aqui esperava para não disputar o banco e a rede com a abertura do app.
#[tauri::command]
pub fn app_ready(state: State<'_, AppState>) {
    let ms = crate::STARTED_AT.get().map_or(0, |t| t.elapsed().as_millis() as u64);
    tracing::info!(ms, "primeira tela desenhada");
    state.mark_ui_ready();
}

#[tauri::command]
pub fn list_providers() -> Vec<ProviderView> {
    providers::views()
}

async fn connect(app: &AppHandle, state: &AppState, provider: &str) -> Result<Connected, AppError> {
    let entry = providers::connectable(provider)?;
    let AuthKind::OAuth2(oauth_provider) = entry.auth else { return Err(AppError::ProviderUnavailable) };
    let cancel = state.begin_connect();
    let result = oauth::authorize(&state.http, oauth_provider, cancel, |url| {
        app.opener().open_url(url, None::<&str>).map_err(|e| {
            tracing::error!(error = %e, "não foi possível abrir o navegador do sistema");
            AppError::AuthFailed
        })
    })
    .await;
    state.end_connect();
    let (tokens, email) = result?;
    let connected = state.store_oauth_account(entry.id, email, tokens).await?;
    sync::spawn_account_sync(app, connected.account.id.clone());
    Ok(connected)
}

#[tauri::command]
pub async fn connect_account(app: AppHandle, state: State<'_, AppState>, provider: String) -> Result<Connected, AppError> {
    connect(&app, &state, &provider).await
}

/// Refaz o login de uma conta que saiu ou perdeu o acesso. Se o provedor devolver outro e-mail,
/// esse vira uma conta nova e a antiga fica como estava.
#[tauri::command]
pub async fn reconnect_account(app: AppHandle, state: State<'_, AppState>, account_id: String) -> Result<Connected, AppError> {
    let id = self::account_id(&account_id)?;
    let account = state.db.call(move |c| store::require_account(c, &id)).await?;
    connect(&app, &state, &account.provider).await
}

#[tauri::command]
pub fn cancel_connect(state: State<'_, AppState>) {
    state.cancel_connect();
}

#[tauri::command]
pub async fn update_account(
    state: State<'_, AppState>,
    account_id: String,
    display_name: Option<String>,
    color: Option<String>,
) -> Result<Account, AppError> {
    let id = self::account_id(&account_id)?;
    let account = state.db.call(move |c| store::update_account(c, &id, display_name.as_deref(), color.as_deref())).await?;
    state.emit_accounts_changed();
    Ok(account)
}

#[tauri::command]
pub async fn move_account(state: State<'_, AppState>, account_id: String, direction: i64) -> Result<Vec<Account>, AppError> {
    let id = self::account_id(&account_id)?;
    if direction != -1 && direction != 1 {
        return Err(AppError::InvalidInput);
    }
    let accounts = state.db.call(move |c| store::move_account(c, &id, direction)).await?;
    state.emit_accounts_changed();
    Ok(accounts)
}

#[tauri::command]
pub async fn sign_out_account(state: State<'_, AppState>, account_id: String, erase: Option<bool>) -> Result<Account, AppError> {
    let id = self::account_id(&account_id)?;
    let account = state.sign_out(&id).await?;
    // "Apagar também os e-mails desta conta guardados neste computador" (US10, CA7).
    if erase == Some(true) {
        privacy::delete_account_data(&state, id).await?;
    }
    Ok(account)
}

#[tauri::command]
pub async fn set_last_folder(state: State<'_, AppState>, folder_id: String) -> Result<(), AppError> {
    let key = FolderKey::decode(&folder_id)?;
    state
        .db
        .call(move |c| {
            store::folder_of_account(c, &key.account, key.folder)?;
            store::set_last_folder(c, &key.account, &key.encode())
        })
        .await
}

// ---------------------------------------------------------------- pastas e lista

fn folder_view(f: &store::FolderRow) -> Folder {
    Folder {
        id: FolderKey { account: f.account_id.clone(), folder: f.id }.encode(),
        role: f.role,
        name: f.display_name.clone(),
        unread: f.unread,
        total: f.total,
    }
}

/// Pastas do banco. Na primeira vez (banco sem pastas da conta), busca a lista no servidor.
#[tauri::command]
pub async fn list_folders(state: State<'_, AppState>, account_id: String) -> Result<Vec<Folder>, AppError> {
    let id = self::account_id(&account_id)?;
    let lookup = id.clone();
    let (account, rows) = state.db.call(move |c| Ok((store::require_account(c, &lookup)?, store::list_folders(c, &lookup)?))).await?;
    if !rows.is_empty() || account.status != AccountStatus::Active {
        return Ok(rows.iter().map(folder_view).collect());
    }
    let handle = state.handle(&id).await?;
    sync::sync_folder_list(&state, &handle).await?;
    let rows = state.db.call(move |c| store::list_folders(c, &id)).await?;
    Ok(rows.iter().map(folder_view).collect())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePage {
    pub items: Vec<MessageHeader>,
    pub next_cursor: Option<String>,
    pub total: u32,
    pub unread: Option<u32>,
    /// Há uma sincronização desta pasta em andamento; o evento `mailbox-updated` avisa o fim.
    pub syncing: bool,
}

#[tauri::command]
pub async fn list_messages(app: AppHandle, state: State<'_, AppState>, folder_id: String, cursor: Option<String>) -> Result<MessagePage, AppError> {
    let key = FolderKey::decode(&folder_id)?;
    let cursor = cursor.as_deref().map(Cursor::decode).transpose()?;
    if let Some(c) = &cursor {
        if c.account != key.account || c.folder != key.folder {
            return Err(AppError::InvalidId);
        }
    }
    let k = key.clone();
    let (account, mut folder) = state
        .db
        .call(move |c| Ok((store::require_account(c, &k.account)?, store::folder_of_account(c, &k.account, k.folder)?)))
        .await?;
    let active = account.status == AccountStatus::Active;
    let handle = state.handle(&key.account).await?;

    // Pasta nunca sincronizada: a primeira página espera a sincronização (CA3).
    if folder.oldest_uid.is_none() && active && handle.try_begin_sync(folder.id) {
        let result = sync::sync_folder(&state, &handle, folder.id).await;
        handle.end_sync(folder.id);
        result?;
        let id = folder.id;
        folder = state.db.call(move |c| store::get_folder(c, id)).await?.ok_or(AppError::InvalidId)?;
    }

    if let Some(c) = &cursor {
        if Some(c.uid_validity) != folder.uidvalidity {
            return Err(AppError::CursorExpired);
        }
    }
    let before = cursor.as_ref().map(|c| c.before);
    let folder_db_id = folder.id;
    let mut rows = state.db.call(move |c| store::list_page(c, folder_db_id, before, PAGE_SIZE + 1)).await?;

    // Paginando além do que está salvo: traz o próximo bloco do servidor (sem rede, segue com o que há).
    if cursor.is_some() && rows.len() <= PAGE_SIZE as usize && !folder.complete && active {
        match sync::fetch_older(&state, &handle, &folder, sync::OLDER_BATCH, Lane::Interactive).await {
            Ok(_) => {
                let id = folder.id;
                let (f, r) = state
                    .db
                    .call(move |c| Ok((store::get_folder(c, id)?, store::list_page(c, id, before, PAGE_SIZE + 1)?)))
                    .await?;
                folder = f.ok_or(AppError::InvalidId)?;
                rows = r;
            }
            Err(AppError::ImapConnection) => tracing::info!("sem conexão; página só com o que está salvo"),
            Err(e) => return Err(e),
        }
    }

    let has_more = rows.len() > PAGE_SIZE as usize || (!folder.complete && active && !rows.is_empty());
    rows.truncate(PAGE_SIZE as usize);
    // Uma linha por conversa (US8, D1): quantidade, participantes e o estado da conversa inteira.
    let keys: Vec<String> = rows.iter().map(|m| m.thread_key.clone()).collect();
    let (account_id, own) = (key.account.clone(), account.email.clone());
    let mut summaries = state.db.call(move |c| store::thread_summaries(c, &account_id, &own, &keys)).await?;
    for m in rows.iter_mut() {
        m.thread = summaries.remove(&m.thread_key).filter(|s| s.count > 1);
        // A linha mostra o estado da conversa: não lida se alguma não foi lida, com estrela se alguma tem.
        if let Some(t) = &m.thread {
            m.flags = MessageFlags { seen: !t.unread, flagged: t.flagged };
        }
    }
    let next_cursor = match (has_more, rows.last(), folder.uidvalidity) {
        (true, Some(last), Some(v)) => Some(Cursor { account: key.account.clone(), folder: folder.id, uid_validity: v, before: last.uid }.encode()),
        _ => None,
    };
    // Primeira página: as mensagens que estão chegando por um mover deste computador vêm no topo
    // (no servidor, a mensagem movida ganha o maior UID da pasta).
    let incoming = if cursor.is_none() {
        let id = folder.id;
        state.db.call(move |c| store::incoming_moves(c, id)).await?
    } else {
        Vec::new()
    };
    let items: Vec<MessageHeader> = incoming
        .into_iter()
        .map(|(from, m)| header_of(&key.account, from, m, None))
        .chain(rows.into_iter().map(|m| header_of(&key.account, folder.id, m, None)))
        .collect();

    let stale = folder.last_sync_at.is_none_or(|t| store::now() - t >= sync::MIN_RESYNC_SECS);
    if cursor.is_none() && active && stale && !handle.is_syncing(folder.id) {
        sync::spawn_folder_sync(&app, key.account.clone(), folder.id);
    }
    Ok(MessagePage {
        total: folder.total.unwrap_or(items.len() as u32),
        unread: folder.unread,
        syncing: active && handle.is_syncing(folder.id),
        items,
        next_cursor,
    })
}

/// Linha da lista para o frontend. `folder` (nome e id) só nos resultados de várias pastas.
fn header_of(account_id: &str, folder_id: i64, m: store::ListedMessage, folder: Option<String>) -> MessageHeader {
    let in_results = folder.is_some();
    MessageHeader {
        id: MessageKey { account: account_id.to_owned(), folder: folder_id, uid: m.uid }.encode(),
        from_name: m.from_name,
        from_address: m.from_address,
        subject: m.subject,
        date: m.date,
        flags: m.flags,
        followup: m.followup,
        snippet: m.snippet.filter(|s| !s.is_empty()),
        has_files: m.has_files,
        has_images: m.has_images,
        thread: m.thread,
        folder_name: folder,
        folder_id: in_results.then(|| FolderKey { account: account_id.to_owned(), folder: folder_id }.encode()),
    }
}

/// "Atualizar": sincroniza a pasta agora. O resultado chega também pelo evento `mailbox-updated`.
#[tauri::command]
pub async fn sync_folder(state: State<'_, AppState>, folder_id: String) -> Result<(), AppError> {
    let key = FolderKey::decode(&folder_id)?;
    let k = key.clone();
    let account = state
        .db
        .call(move |c| {
            store::folder_of_account(c, &k.account, k.folder)?;
            store::require_account(c, &k.account)
        })
        .await?;
    require_active(&account)?;
    sync::run_folder_sync(&state, &key.account, key.folder).await
}

// ---------------------------------------------------------------- leitura e flags

#[tauri::command]
pub async fn get_message(state: State<'_, AppState>, message_id: String, allow_remote_images: bool) -> Result<MessageView, AppError> {
    let key = MessageKey::decode(&message_id)?;
    let k = key.clone();
    let (account, folder, uid, stored, cached) = state
        .db
        .call(move |c| {
            let account = store::require_account(c, &k.account)?;
            let folder = store::folder_of_account(c, &k.account, k.folder)?;
            let stored = store::message_for_key(c, folder.id, k.uid)?.ok_or(AppError::MessageNotFound)?;
            let cached = store::load_body(c, stored.id)?;
            // Já saiu desta pasta no servidor (mover deste computador): lê de onde ela está agora.
            if store::message_by_uid(c, folder.id, k.uid)?.is_some() {
                return Ok((account, folder, k.uid, stored, cached));
            }
            let (where_now, uid) = store::location_of(c, stored.id, folder.id)?.ok_or(AppError::MessageMoving)?;
            let folder = store::get_folder(c, where_now)?.ok_or(AppError::MessageMoving)?;
            Ok((account, folder, uid, stored, cached))
        })
        .await?;
    if stored.body_state == "too_large" {
        return Err(AppError::MessageTooLarge);
    }
    let handle = state.handle(&key.account).await?;

    // Corpo já salvo: abre sem rede. A mensagem aparece na hora como lida e o servidor recebe o "lida" em
    // seguida, sem segurar a tela; se ele não receber, a próxima sincronização traz o estado do servidor.
    if let Some(raw) = cached {
        let mut view = view_of(&state, &raw, key.encode(), stored.flags, None, allow_remote_images).await;
        if !stored.flags.seen && account.status == AccountStatus::Active {
            let id = stored.id;
            let local = MessageFlags { seen: true, ..stored.flags };
            state.db.call(move |c| store::set_local_flags(c, id, local)).await?;
            view.flags = local;
            state.emit_accounts_changed();
            let app = state.app.clone();
            let (mailbox, v) = (folder.imap_name.clone(), folder.uidvalidity.unwrap_or(0));
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                let marked = imap_client::with_session(&state, &handle, Lane::Interactive, |s| {
                    let mailbox = mailbox.clone();
                    async move {
                        imap_client::select_checked(s, &mailbox, v).await?;
                        imap_client::store_flags(s, &[uid], Some(true), None).await
                    }
                    .boxed()
                })
                .await;
                match marked.map(|m| m.get(&uid).copied()) {
                    Ok(Some(flags)) => {
                        let _ = state.db.call(move |c| store::set_local_flags(c, id, flags)).await;
                    }
                    Ok(None) => {}
                    Err(e) => tracing::info!(kind = e.kind(), "o \"lida\" fica para a próxima sincronização"),
                }
            });
        }
        return Ok(view);
    }

    require_active(&account)?;
    let uidvalidity = folder.uidvalidity.ok_or(AppError::MessageNotFound)?;
    let mailbox = folder.imap_name.clone();
    let fetched = imap_client::with_session(&state, &handle, Lane::Interactive, |s| {
        let mailbox = mailbox.clone();
        async move {
            imap_client::select_checked(s, &mailbox, uidvalidity).await?;
            let raw = imap_client::fetch_raw(s, uid).await?;
            // Abrir marca como lida, só depois de o corpo ter carregado.
            let flags = if raw.flags.seen {
                raw.flags
            } else {
                imap_client::store_flags(s, &[uid], Some(true), None).await?.get(&uid).copied().unwrap_or(raw.flags)
            };
            Ok((raw, flags))
        }
        .boxed()
    })
    .await;
    let (raw, flags) = match fetched {
        Ok(v) => v,
        Err(AppError::MessageTooLarge) => {
            let id = stored.id;
            state.db.call(move |c| store::mark_too_large(c, id)).await?;
            return Err(AppError::MessageTooLarge);
        }
        Err(e) => return Err(e),
    };
    let view = view_of(&state, &raw.raw, key.encode(), flags, raw.internal_date.clone(), allow_remote_images).await;
    // O banco guarda os anexos da mensagem como veio do servidor (cifrada, se for o caso).
    let attachments = message::attachments_of(&raw.raw);
    let preview = crate::preview::from_raw(&raw.raw);
    let id = stored.id;
    state
        .db
        .call(move |c| {
            store::save_body_with(c, id, &raw.raw, &attachments, &preview)?;
            store::set_local_flags(c, id, flags)
        })
        .await?;
    if flags.seen != stored.flags.seen {
        state.emit_accounts_changed();
    }
    Ok(view)
}

/// Monta a mensagem exibida, decifrando e verificando o OpenPGP quando houver (US13, CA7).
async fn view_of(state: &AppState, raw: &[u8], id: String, flags: MessageFlags, internal_date: Option<String>, allow_remote_images: bool) -> MessageView {
    let opened = crate::e2ee::open(state, raw.to_vec()).await;
    let shown = if opened.raw.is_empty() { raw } else { &opened.raw };
    let mut view = message::build_view(shown, id, flags, internal_date, allow_remote_images);
    if let Some(subject) = opened.subject {
        view.subject = subject;
    }
    view.security = opened.security;
    view
}

#[derive(Debug, Serialize, Clone)]
pub struct FlagResult {
    pub id: String,
    pub flags: MessageFlags,
}

#[tauri::command]
pub async fn set_flags(state: State<'_, AppState>, message_ids: Vec<String>, seen: Option<bool>, flagged: Option<bool>) -> Result<Vec<FlagResult>, AppError> {
    if message_ids.is_empty() || message_ids.len() > MAX_FLAG_IDS || (seen.is_none() && flagged.is_none()) {
        return Err(AppError::InvalidId);
    }
    let keys = message_ids.iter().map(|id| MessageKey::decode(id)).collect::<Result<Vec<_>, _>>()?;
    let account_id = keys[0].account.clone();
    if keys.iter().any(|k| k.account != account_id) {
        return Err(AppError::InvalidId);
    }
    let mut groups: BTreeMap<i64, Vec<u32>> = BTreeMap::new();
    for k in &keys {
        groups.entry(k.folder).or_default().push(k.uid);
    }
    // Lida, não lida e estrela numa linha da lista valem para a conversa na pasta (US8).
    let wanted: Vec<(i64, Vec<u32>)> = groups.into_iter().collect();
    let groups: BTreeMap<i64, Vec<u32>> = state
        .db
        .call(move |c| wanted.into_iter().map(|(f, uids)| Ok((f, store::with_thread_uids(c, f, &uids)?))).collect::<Result<_, AppError>>())
        .await?;
    let mut groups = groups;
    let id = account_id.clone();
    let folder_ids: Vec<i64> = groups.keys().copied().collect();
    let (account, folders) = state
        .db
        .call(move |c| {
            let account = store::require_account(c, &id)?;
            let folders = folder_ids.iter().map(|f| store::folder_of_account(c, &id, *f)).collect::<Result<Vec<_>, _>>()?;
            Ok((account, folders))
        })
        .await?;
    require_active(&account)?;
    let handle = state.handle(&account_id).await?;

    let mut results = Vec::with_capacity(keys.len());
    for folder in folders {
        let uids = groups.remove(&folder.id).unwrap_or_default();
        let uidvalidity = folder.uidvalidity.ok_or(AppError::MessageNotFound)?;
        let mailbox = folder.imap_name.clone();
        let wanted = uids.clone();
        let confirmed = imap_client::with_session(&state, &handle, Lane::Interactive, |s| {
            let mailbox = mailbox.clone();
            let wanted = wanted.clone();
            async move {
                imap_client::select_checked(s, &mailbox, uidvalidity).await?;
                imap_client::store_flags(s, &wanted, seen, flagged).await
            }
            .boxed()
        })
        .await?;
        let folder_id = folder.id;
        let updates: Vec<(u32, MessageFlags)> = uids.iter().filter_map(|u| confirmed.get(u).map(|f| (*u, *f))).collect();
        let to_store = updates.clone();
        state
            .db
            .call(move |c| {
                for (uid, flags) in &to_store {
                    if let Some(m) = store::message_by_uid(c, folder_id, *uid)? {
                        store::set_local_flags(c, m.id, *flags)?;
                    }
                }
                Ok(())
            })
            .await?;
        for (uid, flags) in updates {
            results.push(FlagResult { id: MessageKey { account: account_id.clone(), folder: folder_id, uid }.encode(), flags });
        }
    }
    if results.len() < keys.len() {
        tracing::info!(requested = keys.len(), updated = results.len(), "algumas mensagens não existem mais");
    }
    state.emit_accounts_changed();
    Ok(results)
}

// ---------------------------------------------------------------- busca (US4, CA1)

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    pub items: Vec<MessageHeader>,
    /// Havia mais resultados que o limite.
    pub truncated: bool,
}

const MIN_QUERY_CHARS: usize = 2;
const MAX_QUERY_CHARS: usize = 200;

#[tauri::command]
pub async fn search_messages(state: State<'_, AppState>, account_id: String, query: String) -> Result<SearchPage, AppError> {
    let id = self::account_id(&account_id)?;
    let query = query.trim().to_owned();
    let chars = query.chars().count();
    if chars < MIN_QUERY_CHARS {
        return Ok(SearchPage { items: Vec::new(), truncated: false });
    }
    if chars > MAX_QUERY_CHARS || query.chars().any(char::is_control) {
        return Err(AppError::InvalidInput);
    }
    let account = id.clone();
    let hits = state.db.call(move |c| {
        store::require_account(c, &account)?;
        store::search(c, &account, &query)
    })
    .await?;
    let page = hits_page(&id, hits);
    tracing::info!(results = page.items.len(), truncated = page.truncated, "busca local");
    Ok(page)
}

/// Resultados com a pasta de cada mensagem (busca e sinalizadas), cortados no limite.
fn hits_page(account_id: &str, mut hits: Vec<store::SearchHit>) -> SearchPage {
    let truncated = hits.len() > store::SEARCH_LIMIT as usize;
    hits.truncate(store::SEARCH_LIMIT as usize);
    let items = hits.into_iter().map(|h| header_of(account_id, h.folder_id, h.message, Some(h.folder_name))).collect();
    SearchPage { items, truncated }
}

// ---------------------------------------------------------------- sinalizadas (US6, CA6)

fn flag_dates(today: String, week_end: String) -> Result<store::FlagDates, AppError> {
    if !store::valid_due_date(&today) || !store::valid_due_date(&week_end) || week_end < today {
        return Err(AppError::InvalidInput);
    }
    Ok(store::FlagDates { today, week_end })
}

#[tauri::command]
pub async fn flagged_counts(state: State<'_, AppState>, account_id: String, today: String, week_end: String) -> Result<store::FlaggedCounts, AppError> {
    let id = self::account_id(&account_id)?;
    let dates = flag_dates(today, week_end)?;
    state
        .db
        .call(move |c| {
            store::require_account(c, &id)?;
            store::flagged_counts(c, &id, &dates)
        })
        .await
}

#[tauri::command]
pub async fn list_flagged(
    state: State<'_, AppState>,
    account_id: String,
    bucket: store::FlagBucket,
    today: String,
    week_end: String,
) -> Result<SearchPage, AppError> {
    let id = self::account_id(&account_id)?;
    let dates = flag_dates(today, week_end)?;
    let account = id.clone();
    let hits = state
        .db
        .call(move |c| {
            store::require_account(c, &account)?;
            store::flagged(c, &account, bucket, &dates)
        })
        .await?;
    let page = hits_page(&id, hits);
    tracing::info!(results = page.items.len(), "sinalizadas listadas");
    Ok(page)
}

// ---------------------------------------------------------------- aparência (US6, CA3–CA4)

#[tauri::command]
pub async fn get_appearance(state: State<'_, AppState>) -> Result<store::Appearance, AppError> {
    state.db.call(|c| store::appearance(c)).await
}

#[tauri::command]
pub async fn set_appearance(state: State<'_, AppState>, change: store::AppearanceChange) -> Result<store::Appearance, AppError> {
    state.db.call(move |c| store::set_appearance(c, &change)).await
}

// ---------------------------------------------------------------- conversa (US8, CA6)

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMessage {
    #[serde(flatten)]
    pub header: MessageHeader,
    /// Enviada pela própria conta ("Você").
    pub mine: bool,
}

/// As mensagens da conversa da mensagem aberta, da mais antiga para a mais nova. O corpo de cada
/// uma continua vindo de `get_message`. A mensagem aberta volta com o mesmo id.
#[tauri::command]
pub async fn get_conversation(state: State<'_, AppState>, message_id: String) -> Result<Vec<ConversationMessage>, AppError> {
    let key = MessageKey::decode(&message_id)?;
    let k = key.clone();
    let (account, row, items) = state
        .db
        .call(move |c| {
            let account = store::require_account(c, &k.account)?;
            let folder = store::folder_of_account(c, &k.account, k.folder)?;
            let row = store::message_for_key(c, folder.id, k.uid)?.ok_or(AppError::MessageNotFound)?.id;
            let items = store::conversation(c, &k.account, &account.email, row)?;
            Ok((account, row, items))
        })
        .await?;
    let messages: Vec<ConversationMessage> = items
        .into_iter()
        .map(|i| {
            let mut header = header_of(&account.id, i.folder_id, i.message, None);
            if i.message_id == row {
                header.id = key.encode();
            }
            ConversationMessage { header, mine: i.mine }
        })
        .collect();
    tracing::info!(count = messages.len(), "conversa aberta");
    Ok(messages)
}

// ---------------------------------------------------------------- notificações (US8, CA4)

#[tauri::command]
pub async fn get_notify_settings(state: State<'_, AppState>) -> Result<store::NotifySettings, AppError> {
    state.db.call(|c| store::notify_settings(c)).await
}

/// Salva as preferências, atualiza o que o módulo de notificações usa e liga ou desliga a abertura
/// com o Windows.
#[tauri::command]
pub async fn set_notify_settings(app: AppHandle, state: State<'_, AppState>, settings: store::NotifySettings) -> Result<store::NotifySettings, AppError> {
    let saved = state.db.call(move |c| store::set_notify_settings(c, &settings)).await?;
    app.state::<crate::notify::Notifier>().set_settings(saved.clone());
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    let result = if saved.autostart { autolaunch.enable() } else { autolaunch.disable() };
    if let Err(e) = result {
        tracing::warn!(error = %e, "não foi possível mudar a abertura com o Windows");
    }
    tracing::info!(autostart = saved.autostart, tray = saved.tray_on_close, "notificações ajustadas");
    Ok(saved)
}

// ---------------------------------------------------------------- download em segundo plano (US9)

/// Pré-busca (CA3): as próximas mensagens da lista, as da conversa e a linha sob o mouse. Volta na
/// hora; o download acontece em segundo plano.
#[tauri::command]
pub async fn prefetch_messages(app: AppHandle, message_ids: Vec<String>) -> Result<(), AppError> {
    let mut keys: BTreeMap<String, Vec<(i64, u32)>> = BTreeMap::new();
    for id in message_ids.iter().take(crate::offline::PREFETCH_LIMIT) {
        let key = MessageKey::decode(id)?;
        keys.entry(key.account).or_default().push((key.folder, key.uid));
    }
    for (account_id, keys) in keys {
        crate::offline::prefetch(&app, account_id, keys);
    }
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineSettings {
    /// "30", "90", "all" ou "none" (CA4).
    pub days: String,
    /// Tamanho do banco local (todas as contas), em bytes.
    pub local_bytes: u64,
    /// Conteúdo guardado fora do período, que pode ser apagado (US10, CA7).
    pub outside_bytes: u64,
}

/// Até que data o conteúdo está "dentro do período": `None` apaga tudo ("Só ao abrir"); com "Tudo",
/// nada fica de fora.
fn outside_cutoff(choice: &str) -> Option<Option<String>> {
    match store::offline_since(choice, chrono::Local::now().date_naive()) {
        None => Some(None),
        Some(None) => None,
        Some(Some(date)) => Some(Some(date)),
    }
}

async fn offline_settings(app: &AppHandle, state: &AppState) -> Result<OfflineSettings, AppError> {
    state
        .db
        .call({
            let local_bytes = local_bytes(app);
            move |c| {
                let days = store::offline_days(c)?;
                let outside_bytes = match outside_cutoff(&days) {
                    Some(before) => store::bytes_outside(c, before.as_deref())?,
                    None => 0,
                };
                Ok(OfflineSettings { days, local_bytes, outside_bytes })
            }
        })
        .await
}

fn local_bytes(app: &AppHandle) -> u64 {
    app.path().app_data_dir().ok().and_then(|dir| std::fs::metadata(dir.join(crate::db::FILE_NAME)).ok()).map_or(0, |m| m.len())
}

#[tauri::command]
pub async fn get_offline_settings(app: AppHandle, state: State<'_, AppState>) -> Result<OfflineSettings, AppError> {
    offline_settings(&app, &state).await
}

/// Apaga o conteúdo guardado fora do período escolhido (US10, CA7, D7) e troca a chave.
#[tauri::command]
pub async fn purge_outside_period(app: AppHandle, state: State<'_, AppState>) -> Result<OfflineSettings, AppError> {
    let purged = state
        .db
        .call(|c| {
            let days = store::offline_days(c)?;
            match outside_cutoff(&days) {
                Some(before) => store::purge_outside(c, before.as_deref()),
                None => Ok(0),
            }
        })
        .await?;
    tracing::info!(count = purged, "conteúdo fora do período apagado");
    privacy::compact_and_rotate(&state).await;
    offline_settings(&app, &state).await
}

/// Troca a chave de proteção dos dados guardados neste computador (US10, CA4).
#[tauri::command]
pub async fn rotate_db_key(state: State<'_, AppState>) -> Result<(), AppError> {
    privacy::rotate_key(&state).await
}

// ---------------------------------------------------------------- criptografia de ponta a ponta (US13)

/// Impressão digital válida: 40 (v4) ou 64 (v6) dígitos hexadecimais.
fn fingerprint(value: &str) -> Result<String, AppError> {
    let v = value.trim().to_uppercase();
    if (v.len() == 40 || v.len() == 64) && v.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(v)
    } else {
        Err(AppError::InvalidId)
    }
}

#[tauri::command]
pub async fn list_pgp_keys(state: State<'_, AppState>) -> Result<Vec<pgp_store::PgpKeyRow>, AppError> {
    state.db.call(|c| pgp_store::list_keys(c)).await
}

/// Cria uma chave para a conta (CA2) e, se a conta ainda não tem uma, passa a usá-la.
#[tauri::command]
pub async fn generate_pgp_key(state: State<'_, AppState>, account_id: String, kind: openpgp::KeyKind, name: String) -> Result<pgp_store::PgpKeyRow, AppError> {
    let id = self::account_id(&account_id)?;
    let lookup = id.clone();
    let account = state.db.call(move |c| store::require_account(c, &lookup)).await?;
    let name = name.trim().chars().take(120).collect::<String>();
    let email = account.email.clone();
    // RSA 4096 leva alguns segundos: fora das threads do runtime.
    let secret = tokio::task::spawn_blocking(move || openpgp::generate(kind, &name, &email)).await.map_err(|_| AppError::Storage)??;
    let public = secret.to_public_key();
    let info = openpgp::info(&public);
    let (pubb, secb) = (openpgp::public_bytes(&public)?, openpgp::secret_bytes(&secret)?);
    let row = state
        .db
        .call(move |c| {
            let row = pgp_store::upsert_key(c, &info, &pubb, Some(&secb), "generated")?;
            let settings = pgp_store::crypto_settings(c, &id)?;
            if settings.pgp_key.is_none() {
                pgp_store::set_crypto_settings(c, &id, &pgp_store::CryptoSettings { pgp_key: Some(row.fingerprint.clone()), ..settings })?;
            }
            Ok(row)
        })
        .await?;
    tracing::info!(kind = ?kind, "chave OpenPGP criada");
    Ok(row)
}

/// Importa chaves de um arquivo ou da área de transferência (texto em armadura, ou binário em base64).
#[tauri::command]
pub async fn import_pgp_keys(state: State<'_, AppState>, data: String, binary: bool, passphrase: Option<String>) -> Result<Vec<pgp_store::PgpKeyRow>, AppError> {
    use base64::Engine;
    if data.len() > 4 * 1024 * 1024 {
        return Err(AppError::InvalidInput);
    }
    let bytes = if binary { base64::engine::general_purpose::STANDARD.decode(data.trim()).map_err(|_| AppError::InvalidInput)? } else { data.into_bytes() };
    let passphrase = passphrase.map(zeroize::Zeroizing::new);
    let rows = state.db.call(move |c| crate::e2ee::import(c, &bytes, passphrase.as_deref().map(String::as_str))).await?;
    tracing::info!(count = rows.len(), "chaves OpenPGP importadas");
    Ok(rows)
}

/// A chave pública em armadura, para copiar ou salvar.
#[tauri::command]
pub async fn export_pgp_public(state: State<'_, AppState>, fingerprint: String) -> Result<String, AppError> {
    let fp = self::fingerprint(&fingerprint)?;
    let bytes = state.db.call(move |c| pgp_store::public_key(c, &fp)).await?.ok_or(AppError::InvalidId)?;
    openpgp::armored_public(&openpgp::public_from_bytes(&bytes)?)
}

/// Backup da minha chave secreta, protegido pela senha dada (CA2).
#[tauri::command]
pub async fn backup_pgp_secret(state: State<'_, AppState>, fingerprint: String, passphrase: String) -> Result<String, AppError> {
    let fp = self::fingerprint(&fingerprint)?;
    let passphrase = zeroize::Zeroizing::new(passphrase);
    let bytes = state.db.call(move |c| pgp_store::secret_key(c, &fp)).await?.ok_or(AppError::InvalidId)?;
    let armored = tokio::task::spawn_blocking(move || openpgp::armored_secret_backup(&openpgp::secret_from_bytes(&bytes)?, &passphrase))
        .await
        .map_err(|_| AppError::Storage)??;
    tracing::info!("backup de chave OpenPGP exportado");
    Ok(armored)
}

#[tauri::command]
pub async fn set_pgp_acceptance(state: State<'_, AppState>, fingerprint: String, acceptance: String) -> Result<pgp_store::PgpKeyRow, AppError> {
    let fp = self::fingerprint(&fingerprint)?;
    state.db.call(move |c| pgp_store::set_acceptance(c, &fp, &acceptance)).await
}

/// Exclui uma chave. Uma chave minha em uso por alguma conta deixa de ser usada.
#[tauri::command]
pub async fn delete_pgp_key(state: State<'_, AppState>, fingerprint: String) -> Result<(), AppError> {
    let fp = self::fingerprint(&fingerprint)?;
    state
        .db
        .call(move |c| {
            for account in store::list_accounts(c)? {
                let s = pgp_store::crypto_settings(c, &account.id)?;
                if s.pgp_key.as_deref() == Some(fp.as_str()) {
                    pgp_store::set_crypto_settings(c, &account.id, &pgp_store::CryptoSettings { pgp_key: None, ..s })?;
                }
            }
            pgp_store::delete_key(c, &fp)
        })
        .await?;
    tracing::info!("chave OpenPGP excluída");
    Ok(())
}

#[tauri::command]
pub async fn get_crypto_settings(state: State<'_, AppState>, account_id: String) -> Result<pgp_store::CryptoSettings, AppError> {
    let id = self::account_id(&account_id)?;
    state.db.call(move |c| pgp_store::crypto_settings(c, &id)).await
}

#[tauri::command]
pub async fn set_crypto_settings(state: State<'_, AppState>, account_id: String, settings: pgp_store::CryptoSettings) -> Result<pgp_store::CryptoSettings, AppError> {
    let id = self::account_id(&account_id)?;
    let fp = settings.pgp_key.as_deref().map(self::fingerprint).transpose()?;
    state.db.call(move |c| pgp_store::set_crypto_settings(c, &id, &pgp_store::CryptoSettings { pgp_key: fp, ..settings })).await
}

/// Quais destinatários têm chave aceita (CA6), para o cadeado de cada um no composer.
#[tauri::command]
pub async fn recipients_key_status(state: State<'_, AppState>, addresses: Vec<String>) -> Result<Vec<crate::e2ee::RecipientKey>, AppError> {
    if addresses.len() > 100 {
        return Err(AppError::InvalidInput);
    }
    let addresses: Vec<String> = addresses.into_iter().map(|a| a.trim().to_lowercase()).collect();
    state.db.call(move |c| crate::e2ee::recipient_status(c, &addresses)).await
}

/// Busca a chave de um e-mail na internet (D5: a interface pede permissão antes) e a guarda como
/// "ainda não aceita".
#[tauri::command]
pub async fn search_pgp_key(state: State<'_, AppState>, email: String) -> Result<Vec<pgp_store::PgpKeyRow>, AppError> {
    let email = email.trim().to_lowercase();
    let Some((bytes, _source)) = crate::key_lookup::search(&state.http, &email).await? else { return Ok(Vec::new()) };
    let keys = openpgp::parse_keys(&bytes)?;
    state
        .db
        .call(move |c| {
            let mut out = Vec::new();
            for key in keys {
                let public = key.public();
                let info = openpgp::info(&public);
                // Só chaves com o e-mail procurado.
                if info.emails.contains(&email) {
                    out.push(pgp_store::upsert_key(c, &info, &openpgp::public_bytes(&public)?, None, "search")?);
                }
            }
            Ok(out)
        })
        .await
}

// ---------------------------------------------------------------- regras (US12)

#[tauri::command]
pub async fn list_rules(state: State<'_, AppState>, account_id: String) -> Result<Vec<store::RuleRow>, AppError> {
    let id = self::account_id(&account_id)?;
    state.db.call(move |c| store::list_rules(c, &id)).await
}

/// Cria (sem `rule_id`) ou muda uma regra. Pastas de destino têm de ser da conta.
#[tauri::command]
pub async fn save_rule(state: State<'_, AppState>, account_id: String, rule_id: Option<i64>, name: String, enabled: bool, spec: rules::RuleSpec) -> Result<store::RuleRow, AppError> {
    let id = self::account_id(&account_id)?;
    for key in spec.folders.iter().chain(spec.actions.iter().filter_map(|a| match a {
        rules::Action::Move { folder } | rules::Action::Copy { folder } => Some(folder),
        _ => None,
    })) {
        if FolderKey::decode(key)?.account != id {
            return Err(AppError::InvalidId);
        }
    }
    let saved = state.db.call(move |c| store::save_rule(c, &id, rule_id, &name, enabled, &spec)).await?;
    tracing::info!(new = rule_id.is_none(), "regra salva");
    Ok(saved)
}

#[tauri::command]
pub async fn delete_rule(state: State<'_, AppState>, account_id: String, rule_id: i64) -> Result<(), AppError> {
    let id = self::account_id(&account_id)?;
    state.db.call(move |c| store::delete_rule(c, &id, rule_id)).await
}

#[tauri::command]
pub async fn set_rule_enabled(state: State<'_, AppState>, account_id: String, rule_id: i64, enabled: bool) -> Result<store::RuleRow, AppError> {
    let id = self::account_id(&account_id)?;
    state.db.call(move |c| store::set_rule_enabled(c, &id, rule_id, enabled)).await
}

#[tauri::command]
pub async fn reorder_rules(state: State<'_, AppState>, account_id: String, rule_ids: Vec<i64>) -> Result<Vec<store::RuleRow>, AppError> {
    let id = self::account_id(&account_id)?;
    state.db.call(move |c| store::reorder_rules(c, &id, &rule_ids)).await
}

/// Prévia do editor (CA2); `all` conta a pasta inteira, para "Rodar agora".
#[tauri::command]
pub async fn preview_rule(state: State<'_, AppState>, account_id: String, rule_id: Option<i64>, spec: rules::RuleSpec, folder_id: Option<String>, all: Option<bool>) -> Result<rule_runner::Preview, AppError> {
    let id = self::account_id(&account_id)?;
    rule_runner::preview(&state, id, rule_id, spec, folder_id, all == Some(true)).await
}

#[tauri::command]
pub async fn run_rule_now(state: State<'_, AppState>, account_id: String, rule_id: i64, folder_id: Option<String>) -> Result<usize, AppError> {
    let id = self::account_id(&account_id)?;
    rule_runner::run_now(&state, id, rule_id, folder_id).await
}

#[tauri::command]
pub async fn rule_history(state: State<'_, AppState>, account_id: String) -> Result<Vec<store::RuleRunRow>, AppError> {
    let id = self::account_id(&account_id)?;
    state.db.call(move |c| store::rule_history(c, &id)).await
}

#[tauri::command]
pub async fn undo_rule_run(state: State<'_, AppState>, account_id: String, run_id: i64) -> Result<usize, AppError> {
    let id = self::account_id(&account_id)?;
    rule_runner::undo(&state, id, run_id).await
}

/// Sugestões para criar uma regra a partir de uma mensagem (CA5).
#[tauri::command]
pub async fn rule_suggestions(state: State<'_, AppState>, message_id: String) -> Result<rule_runner::RuleSuggestions, AppError> {
    let key = MessageKey::decode(&message_id)?;
    state.db.call(move |c| rule_runner::suggestions(c, &key.account, key.folder, key.uid)).await
}

// ---------------------------------------------------------------- privacidade (US10)

#[tauri::command]
pub fn lock_status(state: State<'_, AppState>) -> LockStatus {
    privacy::status(&state)
}

#[tauri::command]
pub async fn unlock_app(state: State<'_, AppState>, password: String) -> Result<(), AppError> {
    privacy::unlock(&state, Zeroizing::new(password)).await
}

#[tauri::command]
pub async fn lock_app(state: State<'_, AppState>) -> Result<(), AppError> {
    privacy::lock_now(&state).await
}

#[tauri::command]
pub async fn set_app_password(state: State<'_, AppState>, password: String) -> Result<(), AppError> {
    privacy::set_password(&state, Zeroizing::new(password)).await
}

#[tauri::command]
pub async fn change_app_password(state: State<'_, AppState>, current: String, password: String) -> Result<(), AppError> {
    privacy::change_password(&state, Zeroizing::new(current), Zeroizing::new(password)).await
}

#[tauri::command]
pub async fn remove_app_password(state: State<'_, AppState>, current: String) -> Result<(), AppError> {
    privacy::remove_password(&state, Zeroizing::new(current)).await
}

/// "Esqueci a senha": só com o app bloqueado.
#[tauri::command]
pub async fn wipe_local_data(state: State<'_, AppState>) -> Result<(), AppError> {
    privacy::wipe(&state).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredByAccount {
    pub account_id: String,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacySettings {
    pub has_password: bool,
    pub lock: store::LockSettings,
    pub stored: Vec<StoredByAccount>,
}

#[tauri::command]
pub async fn get_privacy_settings(state: State<'_, AppState>) -> Result<PrivacySettings, AppError> {
    let (lock, stored) = state.db.call(|c| Ok((store::lock_settings(c)?, store::stored_bytes_by_account(c)?))).await?;
    Ok(PrivacySettings {
        has_password: state.lock.has_password(),
        lock,
        stored: stored.into_iter().map(|(account_id, bytes)| StoredByAccount { account_id, bytes }).collect(),
    })
}

#[tauri::command]
pub async fn set_lock_settings(state: State<'_, AppState>, minutes: String, with_windows: bool) -> Result<store::LockSettings, AppError> {
    state.db.call(move |c| store::set_lock_settings(c, &store::LockSettings { minutes, with_windows })).await
}

/// O disco do Epistula está cifrado (BitLocker)? `None` quando não dá para saber (US10, CA6).
#[tauri::command]
pub async fn disk_encryption(app: AppHandle) -> Result<Option<bool>, AppError> {
    Ok(privacy::disk_encrypted(privacy::data_dir(&app)?).await)
}

/// Apaga deste computador os dados de uma conta de que o usuário saiu (US10, CA7).
#[tauri::command]
pub async fn delete_account_data(state: State<'_, AppState>, account_id: String) -> Result<(), AppError> {
    let id = self::account_id(&account_id)?;
    privacy::delete_account_data(&state, id).await
}

/// Trocar o período não apaga nada já salvo (D4); um período maior acorda a fila na hora.
#[tauri::command]
pub async fn set_offline_days(app: AppHandle, state: State<'_, AppState>, days: String) -> Result<OfflineSettings, AppError> {
    state.db.call(move |c| store::set_offline_days(c, &days)).await?;
    crate::offline::wake(&app);
    let settings = offline_settings(&app, &state).await?;
    tracing::info!(days = settings.days.as_str(), "período de download ajustado");
    Ok(settings)
}

// ---------------------------------------------------------------- sino (US8, CA7)

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticeList {
    pub items: Vec<store::Notice>,
    pub unseen: usize,
    /// Não perturbe em efeito agora (horário ou 1 hora): o sino fica riscado.
    pub quiet: bool,
    /// "Não perturbe por 1 hora" ligado.
    pub snoozed: bool,
}

#[tauri::command]
pub async fn list_notices(app: AppHandle, state: State<'_, AppState>) -> Result<NoticeList, AppError> {
    let at = store::now();
    let items = state.db.call(move |c| store::list_notices(c, at)).await?;
    let n = app.state::<crate::notify::Notifier>();
    Ok(NoticeList { unseen: items.iter().filter(|i| !i.seen).count(), items, quiet: n.quiet_now(), snoozed: n.snoozed() })
}

#[tauri::command]
pub async fn mark_notices_seen(state: State<'_, AppState>) -> Result<(), AppError> {
    state.db.call(|c| store::mark_notices_seen(c)).await
}

/// "Não perturbe por 1 hora" do sino; o mesmo da bandeja.
#[tauri::command]
pub async fn set_snooze(app: AppHandle, on: bool) -> Result<bool, AppError> {
    let on = app.state::<crate::notify::Notifier>().set_snooze(on);
    crate::tray::sync_snooze(&app);
    let _ = app.emit("notices-updated", ());
    tracing::info!(on, "não perturbe do sino");
    Ok(on)
}

// ---------------------------------------------------------------- menu da conta (US7, CA6)

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountOverview {
    #[serde(flatten)]
    pub counts: store::AccountOverview,
    /// Tamanho do banco local (todas as contas), em bytes.
    pub local_bytes: u64,
}

#[tauri::command]
pub async fn account_overview(app: AppHandle, state: State<'_, AppState>, account_id: String) -> Result<AccountOverview, AppError> {
    let id = self::account_id(&account_id)?;
    let counts = state.db.call(move |c| store::account_overview(c, &id)).await?;
    Ok(AccountOverview { counts, local_bytes: local_bytes(&app) })
}

/// "Sincronizar agora" do menu da conta: lista de pastas e Caixa de entrada.
#[tauri::command]
pub async fn sync_account(state: State<'_, AppState>, account_id: String) -> Result<(), AppError> {
    let id = self::account_id(&account_id)?;
    let lookup = id.clone();
    let account = state.db.call(move |c| store::require_account(c, &lookup)).await?;
    require_active(&account)?;
    sync::run_account_sync(&state, &id).await
}

// ---------------------------------------------------------------- mover, excluir, arquivar (CA4, CA5)

/// Mensagens agrupadas por pasta, todas da mesma conta, com a conta ativa.
async fn resolve_messages(state: &AppState, message_ids: &[String]) -> Result<(Account, Vec<(FolderRow, Vec<u32>)>), AppError> {
    if message_ids.is_empty() || message_ids.len() > MAX_FLAG_IDS {
        return Err(AppError::InvalidId);
    }
    let keys = message_ids.iter().map(|id| MessageKey::decode(id)).collect::<Result<Vec<_>, _>>()?;
    let account_id = keys[0].account.clone();
    if keys.iter().any(|k| k.account != account_id) {
        return Err(AppError::InvalidId);
    }
    let mut groups: BTreeMap<i64, Vec<u32>> = BTreeMap::new();
    for k in &keys {
        groups.entry(k.folder).or_default().push(k.uid);
    }
    let (account, folders) = state
        .db
        .call(move |c| {
            let account = store::require_account(c, &account_id)?;
            let folders = groups
                .into_iter()
                .map(|(f, uids)| {
                    let folder = store::folder_of_account(c, &account_id, f)?;
                    let uids = store::with_thread_uids(c, folder.id, &uids)?;
                    Ok((folder, uids))
                })
                .collect::<Result<Vec<_>, AppError>>()?;
            Ok((account, folders))
        })
        .await?;
    require_active(&account)?;
    Ok((account, folders))
}

/// Apaga de vez (excluir dentro da Lixeira), pasta por pasta. Uma pasta que falha não impede as
/// outras (US5, CA2): o resultado diz quais ids saíram e quais ficaram. Se nada saiu, devolve o erro.
async fn expunge_from_folders(state: &AppState, account: &Account, groups: Vec<(FolderRow, Vec<u32>)>) -> Result<Moved, AppError> {
    let mut result = Moved::default();
    if groups.is_empty() {
        return Ok(result);
    }
    let handle = state.handle(&account.id).await?;
    let mut first_error = None;
    for (folder, uids) in groups {
        let folder_id = folder.id;
        let ids = |uids: &[u32]| uids.iter().map(|&uid| MessageKey { account: account.id.clone(), folder: folder_id, uid }.encode()).collect::<Vec<_>>();
        let Some(uidvalidity) = folder.uidvalidity else {
            result.failed.extend(ids(&uids));
            first_error.get_or_insert(AppError::MessageNotFound);
            continue;
        };
        let mailbox = folder.imap_name.clone();
        let wanted = uids.clone();
        let outcome = imap_client::with_session(state, &handle, Lane::Interactive, |s| {
            let mailbox = mailbox.clone();
            let wanted = wanted.clone();
            async move {
                imap_client::select_checked(s, &mailbox, uidvalidity).await?;
                imap_client::expunge_uids(s, &wanted).await
            }
            .boxed()
        })
        .await;
        let outcome = match outcome {
            Ok(()) => {
                let removed = uids.clone();
                state.db.call(move |c| store::unlink(c, folder_id, &removed, false)).await.map(drop)
            }
            Err(e) => Err(e),
        };
        match outcome {
            Ok(()) => {
                tracing::info!(role = folder.role.as_str(), count = uids.len(), "mensagens apagadas de vez");
                result.ids.extend(ids(&uids));
            }
            Err(e) => {
                tracing::warn!(role = folder.role.as_str(), count = uids.len(), error = ?e, "falha ao apagar mensagens");
                result.failed.extend(ids(&uids));
                first_error.get_or_insert(e);
            }
        }
    }
    state.emit_accounts_changed();
    match first_error {
        Some(e) if result.ids.is_empty() => Err(e),
        _ => Ok(result),
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Moved {
    /// Lote do mover pendente, para o "Desfazer" (`None` quando nada ficou pendente).
    pub batch: Option<String>,
    /// Saíram da pasta de origem.
    pub ids: Vec<String>,
    /// Continuam na pasta (já estão sendo movidas no servidor, ou o servidor recusou).
    pub failed: Vec<String>,
}

fn new_batch() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Registra o mover (US7, D1): sai da origem e entra no destino na hora; o servidor recebe o MOVE
/// depois do prazo do "Desfazer" (`moves`).
async fn move_later(app: &AppHandle, state: &AppState, account: &Account, groups: Vec<(FolderRow, Vec<u32>)>, to: i64) -> Result<Moved, AppError> {
    if groups.is_empty() {
        return Ok(Moved::default());
    }
    let requested: Vec<(i64, u32)> = groups.iter().flat_map(|(f, uids)| uids.iter().map(move |u| (f.id, *u))).collect();
    let (account_id, batch, wanted) = (account.id.clone(), new_batch(), requested.clone());
    let due = store::now() + moves::UNDO_SECS;
    let lot = batch.clone();
    let accepted = state.db.call(move |c| store::create_pending_moves(c, &account_id, &lot, &wanted, to, due)).await?;
    let key = |(folder, uid): (i64, u32)| MessageKey { account: account.id.clone(), folder, uid }.encode();
    let failed = requested.iter().filter(|r| !accepted.contains(r)).map(|r| key(*r)).collect();
    tracing::info!(count = accepted.len(), skipped = requested.len() - accepted.len(), "mover registrado");
    if !accepted.is_empty() {
        moves::schedule(app);
    }
    state.emit_accounts_changed();
    Ok(Moved { batch: (!accepted.is_empty()).then_some(batch), ids: accepted.into_iter().map(key).collect(), failed })
}

#[tauri::command]
pub async fn move_messages(app: AppHandle, state: State<'_, AppState>, message_ids: Vec<String>, target_folder_id: String) -> Result<Moved, AppError> {
    let target = FolderKey::decode(&target_folder_id)?;
    let (account, groups) = resolve_messages(&state, &message_ids).await?;
    if target.account != account.id {
        return Err(AppError::InvalidId);
    }
    let (target_row, account_id) = (target.folder, account.id.clone());
    let folder = state.db.call(move |c| store::folder_of_account(c, &account_id, target_row)).await?;
    // "Com estrela" no Gmail é a estrela, não uma pasta de destino.
    if folder.role == Role::Starred {
        return Err(AppError::InvalidInput);
    }
    let moved = move_later(&app, &state, &account, groups, folder.id).await?;
    let (account_id, folder_id) = (account.id.clone(), folder.id);
    state.db.call(move |c| store::touch_folder_usage(c, &account_id, folder_id)).await?;
    Ok(moved)
}

/// Excluir: vai para a Lixeira (com "Desfazer"); dentro da Lixeira, apaga de vez (o frontend confirma antes).
#[tauri::command]
pub async fn delete_messages(app: AppHandle, state: State<'_, AppState>, message_ids: Vec<String>) -> Result<Moved, AppError> {
    let (account, groups) = resolve_messages(&state, &message_ids).await?;
    let account_id = account.id.clone();
    let trash = state.db.call(move |c| store::folder_by_role(c, &account_id, Role::Trash)).await?.ok_or(AppError::FolderNotFound)?;
    let (in_trash, elsewhere): (Vec<_>, Vec<_>) = groups.into_iter().partition(|(f, _)| f.id == trash.id);
    let mut moved = move_later(&app, &state, &account, elsewhere, trash.id).await?;
    match expunge_from_folders(&state, &account, in_trash).await {
        Ok(purged) => {
            moved.ids.extend(purged.ids);
            moved.failed.extend(purged.failed);
            Ok(moved)
        }
        Err(e) if moved.ids.is_empty() => Err(e),
        Err(_) => Ok(moved),
    }
}

/// Arquivar: no Gmail, tira da pasta atual e deixa em "Todos os e-mails".
#[tauri::command]
pub async fn archive_messages(app: AppHandle, state: State<'_, AppState>, message_ids: Vec<String>) -> Result<Moved, AppError> {
    let (account, groups) = resolve_messages(&state, &message_ids).await?;
    let account_id = account.id.clone();
    let all = state.db.call(move |c| store::folder_by_role(c, &account_id, Role::All)).await?.ok_or(AppError::FolderNotFound)?;
    let groups = groups.into_iter().filter(|(f, _)| f.id != all.id).collect();
    move_later(&app, &state, &account, groups, all.id).await
}

/// "Desfazer" de mover, arquivar ou excluir: devolve os ids que voltaram para a origem. Os que já
/// estavam no servidor não voltam (o frontend avisa).
#[tauri::command]
pub async fn undo_move(state: State<'_, AppState>, account_id: String, batch: String) -> Result<Vec<String>, AppError> {
    let id = self::account_id(&account_id)?;
    if batch.len() != 24 || !batch.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::InvalidId);
    }
    let restored = state.db.call(move |c| store::cancel_pending(c, &batch)).await?;
    tracing::info!(count = restored.len(), "mover desfeito");
    state.emit_accounts_changed();
    Ok(restored.into_iter().map(|(folder, uid)| MessageKey { account: id.clone(), folder, uid }.encode()).collect())
}

// ---------------------------------------------------------------- envio (US7, CA1–CA3)

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendRequest {
    pub account_id: String,
    pub kind: DraftKind,
    pub message_id: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub keep_attachments: Vec<usize>,
    /// Criptografar e assinar com OpenPGP (US13).
    #[serde(default)]
    pub encrypt: bool,
    #[serde(default)]
    pub sign: bool,
}

/// Valida e agenda o envio para depois do prazo do "Desfazer". O resultado chega pelo evento
/// `send-finished`. Devolve o id para `cancel_send`.
#[tauri::command]
pub async fn send_message(app: AppHandle, state: State<'_, AppState>, request: SendRequest) -> Result<String, AppError> {
    let id = self::account_id(&request.account_id)?;
    reply_kind(request.kind, &request.message_id)?;
    let lookup = id.clone();
    let account = state.db.call(move |c| store::require_account(c, &lookup)).await?;
    require_active(&account)?;
    let original = match &request.message_id {
        Some(m) => {
            let key = MessageKey::decode(m)?;
            Some((message_row(&state, &id, m).await?, key.folder))
        }
        None => None,
    };
    let mut outgoing = send::validate(
        id.clone(),
        request.kind,
        original,
        &request.to,
        &request.cc,
        &request.bcc,
        &request.subject,
        request.body,
        request.keep_attachments,
    )?;
    // Mensagem criptografada é sempre assinada (US13, CA1).
    outgoing.encrypt = request.encrypt;
    outgoing.sign = request.sign || request.encrypt;
    // Sem a chave de algum destinatário, o erro aparece já, e não depois do prazo do "Desfazer".
    if outgoing.encrypt || outgoing.sign {
        let recipients: Vec<String> = outgoing.to.iter().chain(&outgoing.cc).chain(&outgoing.bcc).map(|m| m.email.to_string().to_lowercase()).collect();
        let encrypt = outgoing.encrypt;
        state.db.call(move |c| crate::e2ee::send_keys(c, &id, &recipients, encrypt).map(|_| ())).await?;
    }
    Ok(send::schedule(&app, outgoing))
}

#[tauri::command]
pub fn cancel_send(send_id: String) -> bool {
    send::cancel(&send_id)
}

#[tauri::command]
pub async fn create_folder(state: State<'_, AppState>, account_id: String, name: String) -> Result<Folder, AppError> {
    let id = self::account_id(&account_id)?;
    let name = store::validate_folder_name(&name)?;
    let (lookup, wanted) = (id.clone(), name.clone());
    let (account, taken) = state.db.call(move |c| Ok((store::require_account(c, &lookup)?, store::folder_name_taken(c, &lookup, &wanted)?))).await?;
    require_active(&account)?;
    if taken {
        return Err(AppError::FolderExists);
    }
    let encoded = mailbox::encode_mailbox_name(&name);
    let handle = state.handle(&id).await?;
    let to_create = encoded.clone();
    imap_client::with_session(&state, &handle, Lane::Interactive, |s| {
        let to_create = to_create.clone();
        async move { imap_client::create_mailbox(s, &to_create).await }.boxed()
    })
    .await?;
    sync::sync_folder_list(&state, &handle).await?;
    let folder = state
        .db
        .call(move |c| Ok(store::list_folders(c, &id)?.into_iter().find(|f| f.imap_name == encoded)))
        .await?
        .ok_or(AppError::FolderNotFound)?;
    tracing::info!("pasta criada");
    Ok(folder_view(&folder))
}

#[tauri::command]
pub async fn recent_folders(state: State<'_, AppState>, account_id: String) -> Result<Vec<Folder>, AppError> {
    let id = self::account_id(&account_id)?;
    let rows = state.db.call(move |c| store::recent_folders(c, &id, 5)).await?;
    Ok(rows.iter().map(folder_view).collect())
}

// ---------------------------------------------------------------- sinalização com prazo (CA7)

#[derive(Debug, Serialize)]
pub struct FollowupResult {
    pub id: String,
    pub flags: MessageFlags,
    pub followup: Option<Followup>,
}

/// `action`: "set" (com `due` em AAAA-MM-DD, ou sem data), "complete" ou "clear", para uma ou
/// várias mensagens (US5, CA2). No servidor, "set" liga a estrela; "complete" e "clear" desligam.
#[tauri::command]
pub async fn set_followup(state: State<'_, AppState>, message_ids: Vec<String>, action: String, due: Option<String>) -> Result<Vec<FollowupResult>, AppError> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Action {
        Set,
        Complete,
        Clear,
    }
    let act = match action.as_str() {
        "set" => Action::Set,
        "complete" => Action::Complete,
        "clear" => Action::Clear,
        _ => return Err(AppError::InvalidInput),
    };
    if act == Action::Set && due.as_deref().is_some_and(|d| !store::valid_due_date(d)) {
        return Err(AppError::InvalidInput);
    }
    let flagged = act == Action::Set;
    let (account, groups) = resolve_messages(&state, &message_ids).await?;
    let handle = state.handle(&account.id).await?;
    let mut results = Vec::new();
    for (folder, uids) in groups {
        let uidvalidity = folder.uidvalidity.ok_or(AppError::MessageNotFound)?;
        let mailbox = folder.imap_name.clone();
        let confirmed = imap_client::with_session(&state, &handle, Lane::Interactive, |s| {
            let mailbox = mailbox.clone();
            let wanted = uids.clone();
            async move {
                imap_client::select_checked(s, &mailbox, uidvalidity).await?;
                imap_client::store_flags(s, &wanted, None, Some(flagged)).await
            }
            .boxed()
        })
        .await?;
        let folder_id = folder.id;
        let due = due.clone();
        let saved = state
            .db
            .call(move |c| {
                let mut out = Vec::new();
                for (uid, flags) in confirmed {
                    let Some(m) = store::message_by_uid(c, folder_id, uid)? else { continue };
                    store::set_local_flags(c, m.id, flags)?;
                    let change = match act {
                        Action::Set => FollowupChange::Set(due.as_deref()),
                        Action::Complete => FollowupChange::Complete,
                        Action::Clear => FollowupChange::Clear,
                    };
                    out.push((uid, flags, store::set_followup(c, m.id, change)?));
                }
                Ok(out)
            })
            .await?;
        results.extend(saved.into_iter().map(|(uid, flags, followup)| FollowupResult {
            id: MessageKey { account: account.id.clone(), folder: folder_id, uid }.encode(),
            flags,
            followup,
        }));
    }
    tracing::info!(count = results.len(), "sinalização alterada");
    Ok(results)
}

// ---------------------------------------------------------------- resposta e rascunhos (CA8)

/// Id da mensagem no banco para um id opaco da conta (rascunhos de resposta e encaminhamento).
async fn message_row(state: &AppState, account_id: &str, message_id: &str) -> Result<i64, AppError> {
    let key = MessageKey::decode(message_id)?;
    if key.account != account_id {
        return Err(AppError::InvalidId);
    }
    state
        .db
        .call(move |c| {
            let folder = store::folder_of_account(c, &key.account, key.folder)?;
            Ok(store::message_for_key(c, folder.id, key.uid)?.ok_or(AppError::MessageNotFound)?.id)
        })
        .await
}

fn reply_kind(kind: DraftKind, message_id: &Option<String>) -> Result<(), AppError> {
    if (kind == DraftKind::New) != message_id.is_none() {
        return Err(AppError::InvalidInput);
    }
    Ok(())
}

/// Destinatários, assunto e mensagem original para responder ou encaminhar a mensagem aberta.
#[tauri::command]
pub async fn compose_template(state: State<'_, AppState>, message_id: String, kind: DraftKind) -> Result<Template, AppError> {
    if kind == DraftKind::New {
        return Err(AppError::InvalidInput);
    }
    let key = MessageKey::decode(&message_id)?;
    let (account, raw) = state
        .db
        .call(move |c| {
            let account = store::require_account(c, &key.account)?;
            let folder = store::folder_of_account(c, &key.account, key.folder)?;
            let m = store::message_for_key(c, folder.id, key.uid)?.ok_or(AppError::MessageNotFound)?;
            Ok((account, store::load_body(c, m.id)?))
        })
        .await?;
    let raw = raw.ok_or(AppError::MessageNotFound)?;
    Ok(compose::template(&raw, kind, &account.email))
}

#[tauri::command]
pub async fn get_draft(state: State<'_, AppState>, account_id: String, kind: DraftKind, message_id: Option<String>) -> Result<Option<Draft>, AppError> {
    let id = self::account_id(&account_id)?;
    reply_kind(kind, &message_id)?;
    let row = match &message_id {
        Some(m) => Some(message_row(&state, &id, m).await?),
        None => None,
    };
    state.db.call(move |c| store::get_draft(c, &id, kind, row)).await
}

#[tauri::command]
pub async fn save_draft(state: State<'_, AppState>, account_id: String, kind: DraftKind, message_id: Option<String>, content: DraftContent) -> Result<Draft, AppError> {
    let id = self::account_id(&account_id)?;
    reply_kind(kind, &message_id)?;
    let row = match &message_id {
        Some(m) => Some(message_row(&state, &id, m).await?),
        None => None,
    };
    state.db.call(move |c| {
        store::require_account(c, &id)?;
        store::save_draft(c, &id, kind, row, &content)
    })
    .await
}

#[tauri::command]
pub async fn delete_draft(state: State<'_, AppState>, account_id: String, kind: DraftKind, message_id: Option<String>) -> Result<(), AppError> {
    let id = self::account_id(&account_id)?;
    reply_kind(kind, &message_id)?;
    let row = match &message_id {
        Some(m) => Some(message_row(&state, &id, m).await?),
        None => None,
    };
    state.db.call(move |c| store::delete_draft(c, &id, kind, row)).await
}

#[tauri::command]
pub async fn set_list_width(state: State<'_, AppState>, account_id: String, width: u32) -> Result<(), AppError> {
    let id = self::account_id(&account_id)?;
    state.db.call(move |c| store::set_list_width(c, &id, width)).await
}

// ---------------------------------------------------------------- sincronização (US5, CA7)

#[tauri::command]
pub async fn get_sync_settings(state: State<'_, AppState>) -> Result<store::SyncSettings, AppError> {
    state.db.call(|c| store::sync_settings(c)).await
}

#[tauri::command]
pub async fn set_default_sync_interval(state: State<'_, AppState>, minutes: Option<u32>) -> Result<store::SyncSettings, AppError> {
    state
        .db
        .call(move |c| {
            store::set_default_interval(c, minutes)?;
            store::sync_settings(c)
        })
        .await
}

#[tauri::command]
pub async fn set_account_sync_interval(
    state: State<'_, AppState>,
    account_ids: Vec<String>,
    minutes: Option<u32>,
    use_default: bool,
) -> Result<store::SyncSettings, AppError> {
    let ids = account_ids.iter().map(|id| account_id(id)).collect::<Result<Vec<_>, _>>()?;
    state
        .db
        .call(move |c| {
            store::set_account_interval(c, &ids, minutes, use_default)?;
            store::sync_settings(c)
        })
        .await
}

/// Esquemas que podem sair do app. Tudo o mais (javascript:, file:, relativos) é recusado.
pub fn external_link(url: &str) -> Result<Url, AppError> {
    if url.len() > MAX_URL_LEN {
        return Err(AppError::UnsupportedLink);
    }
    let parsed = Url::parse(url.trim()).map_err(|_| AppError::UnsupportedLink)?;
    match parsed.scheme() {
        "http" | "https" | "mailto" => Ok(parsed),
        _ => Err(AppError::UnsupportedLink),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkOpened {
    pub opened: bool,
    /// O texto do link mostra outro domínio: só abre com `confirmed` (US10, CA8).
    pub warning: Option<links::Misleading>,
}

/// Abre um link do corpo da mensagem no navegador (ou cliente de e-mail) do sistema, sem os
/// parâmetros de rastreamento (US10, D8). `text` é o texto do link: se ele mostra outro domínio, o
/// link só abre depois de o usuário confirmar (`confirmed`).
#[tauri::command]
pub fn open_external_link(app: AppHandle, url: String, text: Option<String>, confirmed: Option<bool>) -> Result<LinkOpened, AppError> {
    let mut parsed = external_link(&url)?;
    let removed = links::strip_tracking(&mut parsed);
    if confirmed != Some(true) {
        if let Some(warning) = text.as_deref().and_then(|t| links::misleading(t, &parsed)) {
            tracing::info!("link leva a outro domínio; pedindo confirmação");
            return Ok(LinkOpened { opened: false, warning: Some(warning) });
        }
    }
    tracing::info!(scheme = parsed.scheme(), tracking_removed = removed, "abrindo link externo");
    app.opener().open_url(parsed.as_str(), None::<&str>).map_err(|e| {
        tracing::warn!(error = %e, "não foi possível abrir o link");
        AppError::UnsupportedLink
    })?;
    Ok(LinkOpened { opened: true, warning: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_and_mail_links_leave_the_app() {
        assert!(external_link("https://lumen.com/contrato").is_ok());
        assert!(external_link("http://lumen.com").is_ok());
        assert!(external_link("mailto:ana@lumen.com").is_ok());
        for bad in ["javascript:alert(1)", " JavaScript:alert(1)", "file:///C:/Windows/system32", "/relativo", "data:text/html,x", "ms-settings:", ""] {
            assert_eq!(external_link(bad), Err(AppError::UnsupportedLink), "{bad}");
        }
        assert_eq!(external_link(&format!("https://a.com/{}", "x".repeat(3000))), Err(AppError::UnsupportedLink));
    }

    #[test]
    fn account_ids_are_validated() {
        assert!(account_id("0f8b1c2d-3e4f-4a5b-8c6d-7e8f9a0b1c2d").is_ok());
        assert_eq!(account_id("x"), Err(AppError::InvalidId));
    }
}
