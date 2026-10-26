//! Sincronização com o servidor (US3, CA3). O servidor é lido numa sessão IMAP só de
//! sincronização e o resultado é aplicado no banco numa transação (`store::apply_snapshot`).
//! O frontend lê sempre do banco e é avisado por eventos quando algo muda.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppError;
use crate::imap_client::{self, ImapSession};
use crate::message::FolderKey;
use crate::session::{AccountHandle, AppState, Lane};
use crate::store::{self, AccountStatus, ApplyReport, FolderRow, Snapshot};

/// Primeira sincronização de uma pasta: as 2.000 mensagens mais recentes (D5 da US3).
pub const INITIAL_WINDOW: u32 = 2000;
/// Cabeçalhos que vêm primeiro, para a lista aparecer logo (US9, CA1 e D3).
pub const FIRST_HEADERS: u32 = 200;
/// O resto da janela chega em pacotes deste tamanho, em segundo plano.
pub const BACKFILL_BATCH: u32 = 250;
/// Mensagens mais antigas buscadas por vez quando o usuário pagina além do que está salvo.
pub const OLDER_BATCH: u32 = 100;
const FETCH_CHUNK: u32 = 250;
/// Abrir uma pasta sincronizada há menos que isso não dispara outra sincronização.
pub const MIN_RESYNC_SECS: i64 = 30;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SavedState {
    pub uidvalidity: Option<u32>,
    pub uidnext: Option<u32>,
    pub highestmodseq: Option<u64>,
    pub oldest_uid: Option<u32>,
}

impl From<&FolderRow> for SavedState {
    fn from(f: &FolderRow) -> Self {
        SavedState { uidvalidity: f.uidvalidity, uidnext: f.uidnext, highestmodseq: f.highestmodseq, oldest_uid: f.oldest_uid }
    }
}

/// Intervalo de sequência da primeira leitura de uma pasta: as `FIRST_HEADERS` mais recentes. O
/// resto, até `INITIAL_WINDOW`, vem depois em pacotes (`backfill_batch`).
pub fn initial_window(exists: u32) -> Option<(u32, u32)> {
    imap_client::page_range(exists, FIRST_HEADERS)
}

/// Quantas mensagens antigas buscar no próximo pacote da primeira sincronização, ou `None` quando a
/// pasta já tem a janela inteira (ou a pasta inteira, se for menor). Continua de onde parou.
pub fn backfill_batch(stored: u32, folder: &FolderRow) -> Option<u32> {
    if folder.complete || folder.oldest_uid.is_none() || stored >= INITIAL_WINDOW {
        return None;
    }
    Some(BACKFILL_BATCH.min(INITIAL_WINDOW - stored))
}

/// Divide `start..=end` em blocos de até `size`.
pub fn chunks(start: u32, end: u32, size: u32) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut a = start;
    while a <= end && size > 0 {
        let b = end.min(a.saturating_add(size - 1));
        out.push((a, b));
        if b == u32::MAX {
            break;
        }
        a = b + 1;
    }
    out
}

/// O que fazer nesta sincronização, a partir do estado salvo e do `SELECT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    /// Pasta nova, UIDVALIDITY mudou ou chegaram mensagens demais desde a última vez.
    Reset,
    Incremental { oldest: u32, new_from: Option<u32>, flags_since: FlagsSince },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagsSince {
    /// Nada mudou desde o último modseq.
    Nothing,
    Modseq(u64),
    /// Servidor sem CONDSTORE ou sem modseq salvo: relê as flags da janela inteira.
    All,
}

pub fn plan(saved: &SavedState, sel: &imap_client::Selected) -> Plan {
    let Some(oldest) = saved.oldest_uid else { return Plan::Reset };
    if saved.uidvalidity != Some(sel.uidvalidity) {
        return Plan::Reset;
    }
    let new_from = match (saved.uidnext, sel.uidnext) {
        (Some(old), Some(new)) if new > old => {
            if new - old > INITIAL_WINDOW {
                return Plan::Reset;
            }
            Some(old)
        }
        (Some(_), Some(_)) => None,
        (None, _) | (_, None) => Some(oldest.saturating_add(1)),
    };
    let flags_since = match (saved.highestmodseq, sel.highestmodseq) {
        (Some(old), Some(new)) if new == old => FlagsSince::Nothing,
        (Some(old), Some(_)) => FlagsSince::Modseq(old),
        _ => FlagsSince::All,
    };
    Plan::Incremental { oldest, new_from, flags_since }
}

async fn remote_snapshot(session: &mut ImapSession, mailbox: &str, saved: &SavedState) -> Result<Snapshot, AppError> {
    let unread = imap_client::unseen_count(session, mailbox).await;
    let sel = imap_client::select(session, mailbox).await?;
    let base = Snapshot {
        uidvalidity: sel.uidvalidity,
        uidnext: sel.uidnext,
        highestmodseq: sel.highestmodseq,
        total: sel.exists,
        unread,
        ..Default::default()
    };
    match plan(saved, &sel) {
        Plan::Reset => {
            let mut headers = Vec::new();
            let window = initial_window(sel.exists);
            if let Some((start, end)) = window {
                for (a, b) in chunks(start, end, FETCH_CHUNK) {
                    headers.extend(imap_client::headers_by_sequence(session, a, b).await?);
                }
            }
            let oldest_uid = headers.iter().map(|h| h.uid).min();
            let complete = window.is_none_or(|(start, _)| start == 1);
            Ok(Snapshot { reset: true, headers, oldest_uid, complete: Some(complete), ..base })
        }
        Plan::Incremental { oldest, new_from, flags_since } => {
            let headers = match new_from {
                Some(from) => imap_client::headers_by_uid(session, &format!("{from}:*"))
                    .await?
                    .into_iter()
                    .filter(|h| h.uid >= from)
                    .collect(),
                None => Vec::new(),
            };
            let flag_updates = match flags_since {
                FlagsSince::Nothing => Vec::new(),
                FlagsSince::Modseq(m) => imap_client::flags_from(session, oldest, Some(m)).await?,
                FlagsSince::All => imap_client::flags_from(session, oldest, None).await?,
            };
            let present = imap_client::search_uids(session, oldest, None).await?;
            Ok(Snapshot { headers, flag_updates, present: Some((oldest, present)), ..base })
        }
    }
}

pub async fn sync_folder(state: &AppState, handle: &AccountHandle, folder_id: i64) -> Result<ApplyReport, AppError> {
    let folder = state.db.call(move |c| store::get_folder(c, folder_id)).await?.ok_or(AppError::InvalidId)?;
    let saved = SavedState::from(&folder);
    let mailbox = folder.imap_name.clone();
    let started = std::time::Instant::now();
    let synced_from = store::now();
    let snap = imap_client::with_session(state, handle, Lane::Sync, |s| {
        let mailbox = mailbox.clone();
        let saved = saved.clone();
        async move { remote_snapshot(s, &mailbox, &saved).await }.boxed()
    })
    .await?;
    let reset = snap.reset;
    let report = state.db.call(move |c| {
        let report = store::apply_snapshot(c, folder_id, &snap)?;
        // Movidas para cá antes desta leitura já chegaram com o UID do servidor (US7, D1).
        store::settle_moves(c, folder_id, synced_from)?;
        Ok(report)
    })
    .await?;
    fill_previews(state, handle, folder_id).await;
    fill_threads(state, handle, folder_id).await;
    tracing::info!(
        role = folder.role.as_str(),
        reset,
        upserted = report.upserted,
        flags = report.flags_changed,
        removed = report.removed,
        ms = started.elapsed().as_millis() as u64,
        "pasta sincronizada"
    );
    Ok(report)
}

/// Conversas completadas por sincronização, para as mensagens salvas antes da US8.
const THREAD_BATCH: u32 = 500;

/// Lê os cabeçalhos de resposta das mensagens que ainda não têm conversa. Falhar aqui não falha a sincronização.
async fn fill_threads(state: &AppState, handle: &AccountHandle, folder_id: i64) {
    let wanted = state
        .db
        .call(move |c| Ok((store::get_folder(c, folder_id)?, store::uids_without_thread(c, folder_id, THREAD_BATCH)?)))
        .await;
    let Ok((Some(folder), uids)) = wanted else { return };
    let Some(uidvalidity) = folder.uidvalidity.filter(|_| !uids.is_empty()) else { return };
    let mailbox = folder.imap_name.clone();
    let fetched = imap_client::with_session(state, handle, Lane::Sync, |s| {
        let (mailbox, uids) = (mailbox.clone(), uids.clone());
        async move {
            imap_client::select_checked(s, &mailbox, uidvalidity).await?;
            imap_client::thread_headers(s, &uids).await
        }
        .boxed()
    })
    .await;
    match fetched {
        Ok(items) => {
            let count = items.len();
            if state.db.call(move |c| store::save_threads(c, folder_id, &items)).await.is_ok() {
                tracing::info!(role = folder.role.as_str(), count, "conversas completadas");
            }
        }
        Err(e) => tracing::info!(kind = e.kind(), "conversas ficaram para a próxima sincronização"),
    }
}

/// Prévias buscadas por sincronização, das mais novas para as mais antigas (US7, D2).
const PREVIEW_BATCH: u32 = 200;

/// Completa a prévia das mensagens da pasta que ainda não têm. Falhar aqui não falha a sincronização.
async fn fill_previews(state: &AppState, handle: &AccountHandle, folder_id: i64) {
    let wanted = state
        .db
        .call(move |c| Ok((store::get_folder(c, folder_id)?, store::uids_without_preview(c, folder_id, PREVIEW_BATCH)?)))
        .await;
    let Ok((Some(folder), uids)) = wanted else { return };
    let Some(uidvalidity) = folder.uidvalidity.filter(|_| !uids.is_empty()) else { return };
    let mailbox = folder.imap_name.clone();
    let fetched = imap_client::with_session(state, handle, Lane::Sync, |s| {
        let (mailbox, uids) = (mailbox.clone(), uids.clone());
        async move {
            imap_client::select_checked(s, &mailbox, uidvalidity).await?;
            imap_client::previews(s, &uids).await
        }
        .boxed()
    })
    .await;
    match fetched {
        Ok(previews) => {
            let count = previews.len();
            if state.db.call(move |c| store::save_previews(c, folder_id, &previews)).await.is_ok() {
                tracing::info!(role = folder.role.as_str(), count, "prévias da lista salvas");
            }
        }
        Err(e) => tracing::info!(kind = e.kind(), "prévias da lista ficaram para a próxima sincronização"),
    }
}

/// Traz do servidor o próximo bloco de `batch` mensagens mais antigas que as salvas. Devolve quantas vieram.
pub async fn fetch_older(state: &AppState, handle: &AccountHandle, folder: &FolderRow, batch: u32, lane: Lane) -> Result<usize, AppError> {
    let (Some(oldest), Some(uidvalidity)) = (folder.oldest_uid, folder.uidvalidity) else { return Ok(0) };
    if folder.complete {
        return Ok(0);
    }
    let mailbox = folder.imap_name.clone();
    let headers = imap_client::with_session(state, handle, lane, |s| {
        let mailbox = mailbox.clone();
        async move {
            imap_client::select_checked(s, &mailbox, uidvalidity).await.map_err(|e| match e {
                AppError::MessageNotFound => AppError::CursorExpired,
                other => other,
            })?;
            if oldest <= 1 {
                return Ok((Vec::new(), true));
            }
            let uids = imap_client::search_uids(s, 1, Some(oldest - 1)).await?;
            let complete = uids.len() <= batch as usize;
            let batch: Vec<u32> = uids.iter().rev().take(batch as usize).copied().collect();
            if batch.is_empty() {
                return Ok((Vec::new(), true));
            }
            Ok((imap_client::headers_by_uid(s, &imap_client::uid_set(&batch)).await?, complete))
        }
        .boxed()
    })
    .await?;
    let (headers, complete) = headers;
    let count = headers.len();
    let snap = Snapshot {
        uidvalidity,
        uidnext: folder.uidnext,
        highestmodseq: folder.highestmodseq,
        total: folder.total.unwrap_or(0),
        unread: None,
        oldest_uid: headers.iter().map(|h| h.uid).min(),
        complete: Some(complete),
        headers,
        ..Default::default()
    };
    let folder_id = folder.id;
    state.db.call(move |c| store::apply_snapshot(c, folder_id, &snap)).await?;
    tracing::info!(role = folder.role.as_str(), count, complete, "mensagens antigas trazidas do servidor");
    Ok(count)
}

pub async fn sync_folder_list(state: &AppState, handle: &AccountHandle) -> Result<(), AppError> {
    let remote = imap_client::with_session(state, handle, Lane::Sync, |s| imap_client::remote_folders(s).boxed()).await?;
    let id = handle.id.clone();
    state.db.call(move |c| store::replace_folders(c, &id, &remote)).await?;
    let _ = state.app.emit("folders-updated", AccountRef { account_id: handle.id.clone() });
    Ok(())
}

// ---------------------------------------------------------------- execução em segundo plano

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountRef {
    account_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MailboxUpdated {
    account_id: String,
    folder_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncStatus {
    account_id: String,
    online: bool,
}

fn report_outcome(state: &AppState, account_id: &str, result: &Result<(), AppError>) {
    let online = match result {
        Ok(()) => true,
        Err(AppError::ImapConnection) => false,
        Err(e) => {
            tracing::info!(kind = e.kind(), "sincronização interrompida");
            return;
        }
    };
    let _ = state.app.emit("sync-status", SyncStatus { account_id: account_id.to_owned(), online });
    crate::notify::connection(&state.app, account_id, online);
}

/// Sincroniza uma pasta (se não houver outra sincronização dela em andamento) e avisa o frontend.
pub async fn run_folder_sync(state: &AppState, account_id: &str, folder_id: i64) -> Result<(), AppError> {
    let handle = state.handle(account_id).await?;
    if !handle.try_begin_sync(folder_id) {
        return Ok(());
    }
    let synced = sync_folder(state, &handle, folder_id).await;
    handle.end_sync(folder_id);
    let result = synced.as_ref().map(|_| ()).map_err(Clone::clone);
    report_outcome(state, account_id, &result);
    // E-mails novos da Caixa de entrada viram notificação (US8, CA1).
    if let Ok(report) = synced {
        let is_inbox = state.db.call(move |c| Ok(store::get_folder(c, folder_id)?.is_some_and(|f| f.role == crate::mailbox::Role::Inbox))).await.unwrap_or(false);
        if is_inbox {
            crate::notify::after_inbox_sync(&state.app, account_id, folder_id, report.new_ids).await;
        }
    }
    if result.is_ok() {
        let folder_key = FolderKey { account: account_id.to_owned(), folder: folder_id }.encode();
        let _ = state.app.emit("mailbox-updated", MailboxUpdated { account_id: account_id.to_owned(), folder_id: folder_key });
        state.emit_accounts_changed();
        // O resto da primeira sincronização e, em seguida, o conteúdo em segundo plano (US9).
        backfill(state, account_id, folder_id).await;
        crate::offline::wake(&state.app);
    }
    result
}

/// Completa a primeira sincronização da pasta até `INITIAL_WINDOW`, um pacote por vez, avisando a
/// lista a cada pacote (US9, CA1). Cede a vez a uma sincronização da mesma pasta em andamento.
async fn backfill(state: &AppState, account_id: &str, folder_id: i64) {
    let Ok(handle) = state.handle(account_id).await else { return };
    loop {
        let loaded = state.db.call(move |c| Ok((store::get_folder(c, folder_id)?, store::stored_in_folder(c, folder_id)?))).await;
        let Ok((Some(folder), stored)) = loaded else { return };
        let Some(batch) = backfill_batch(stored, &folder) else { return };
        if !handle.try_begin_sync(folder_id) {
            return;
        }
        let fetched = fetch_older(state, &handle, &folder, batch, Lane::Sync).await;
        handle.end_sync(folder_id);
        match fetched {
            Ok(n) => {
                let folder_key = FolderKey { account: account_id.to_owned(), folder: folder_id }.encode();
                let _ = state.app.emit("mailbox-updated", MailboxUpdated { account_id: account_id.to_owned(), folder_id: folder_key });
                if n == 0 {
                    return;
                }
            }
            Err(e) => {
                tracing::info!(kind = e.kind(), "primeira sincronização continua na próxima vez");
                return;
            }
        }
    }
}

pub fn spawn_folder_sync(app: &AppHandle, account_id: String, folder_id: i64) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let _ = run_folder_sync(&state, &account_id, folder_id).await;
    });
}

/// Lista de pastas e Caixa de entrada de uma conta ativa.
pub async fn run_account_sync(state: &AppState, account_id: &str) -> Result<(), AppError> {
    let handle = state.handle(account_id).await?;
    let listed = sync_folder_list(state, &handle).await;
    report_outcome(state, account_id, &listed);
    listed?;
    let id = account_id.to_owned();
    if let Some(inbox) = state.db.call(move |c| store::inbox_of(c, &id)).await? {
        run_folder_sync(state, account_id, inbox.id).await?;
    }
    Ok(())
}

pub fn spawn_account_sync(app: &AppHandle, account_id: String) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let _ = run_account_sync(&state, &account_id).await;
    });
}

/// O agendador acorda a cada 15 s e sincroniza as contas cuja vez chegou (US5, CA7).
const SCHEDULER_TICK: Duration = Duration::from_secs(15);
/// Quanto a sincronização automática espera pela primeira tela antes de começar assim mesmo.
const UI_READY_LIMIT: Duration = Duration::from_secs(10);

/// Contas ativas cuja próxima verificação venceu. Conta nunca verificada vence na hora (abertura
/// do app); "Manualmente" (`None`) nunca vence.
pub fn due_accounts(
    accounts: &[store::Account],
    settings: &store::SyncSettings,
    last_run: &std::collections::HashMap<String, std::time::Instant>,
    now: std::time::Instant,
) -> Vec<String> {
    accounts
        .iter()
        .filter(|a| a.status == AccountStatus::Active)
        .filter_map(|a| {
            let minutes = settings
                .accounts
                .iter()
                .find(|s| s.account_id == a.id)
                .map_or(settings.default_minutes, |s| s.minutes)?;
            let due = last_run
                .get(&a.id)
                .is_none_or(|t| now.saturating_duration_since(*t) >= Duration::from_secs(u64::from(minutes) * 60));
            due.then(|| a.id.clone())
        })
        .collect()
}

/// Sincroniza a Caixa de entrada de cada conta ativa no intervalo dela. Mudar o intervalo vale a
/// partir da próxima volta, sem reiniciar. Cada conta roda sozinha (uma sincronização por vez).
pub fn start_periodic(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // A tela vem primeiro (a pasta aberta já sincroniza sozinha ao listar).
        app.state::<AppState>().wait_ui_ready(UI_READY_LIMIT).await;
        app.state::<AppState>().wait_started().await;
        let running: Arc<std::sync::Mutex<HashSet<String>>> = Arc::default();
        let mut last_run = std::collections::HashMap::new();
        loop {
            let state = app.state::<AppState>();
            let loaded = state.db.call(|c| Ok((store::list_accounts(c)?, store::sync_settings(c)?))).await;
            if let Ok((accounts, settings)) = loaded {
                let now = std::time::Instant::now();
                for id in due_accounts(&accounts, &settings, &last_run, now) {
                    if !running.lock().map(|mut r| r.insert(id.clone())).unwrap_or(false) {
                        continue;
                    }
                    last_run.insert(id.clone(), now);
                    let (app, running) = (app.clone(), running.clone());
                    tauri::async_runtime::spawn(async move {
                        let state = app.state::<AppState>();
                        let _ = run_account_sync(&state, &id).await;
                        if let Ok(mut r) = running.lock() {
                            r.remove(&id);
                        }
                    });
                }
            }
            tokio::time::sleep(SCHEDULER_TICK).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imap_client::Selected;
    use crate::mailbox::Role;
    use crate::message::MessageFlags;
    use crate::store::tests::{account, folder, header, test_db};

    fn sel(uidvalidity: u32, uidnext: u32, exists: u32, modseq: Option<u64>) -> Selected {
        Selected { uidvalidity, uidnext: Some(uidnext), exists, highestmodseq: modseq }
    }

    #[test]
    fn scheduler_picks_only_due_active_accounts() {
        use std::collections::HashMap;
        use std::time::Instant;
        let t = test_db();
        let (accounts, ids) = t
            .db
            .with(|c| {
                let a = account(c, "a@gmail.com");
                let b = account(c, "b@gmail.com");
                let m = account(c, "m@gmail.com");
                let out = account(c, "out@gmail.com");
                store::set_account_interval(c, std::slice::from_ref(&b.id), Some(1), false).unwrap();
                store::set_account_interval(c, std::slice::from_ref(&m.id), None, false).unwrap();
                store::set_status(c, &out.id, AccountStatus::SignedOut).unwrap();
                Ok((store::list_accounts(c).unwrap(), [a.id, b.id, m.id, out.id]))
            })
            .unwrap();
        let settings = t.db.with(|c| store::sync_settings(c)).unwrap();
        let [a, b, m, _] = ids;
        let now = Instant::now() + Duration::from_secs(3600);

        // Nunca verificadas: todas as ativas, menos a manual e a que saiu.
        let mut due = due_accounts(&accounts, &settings, &HashMap::new(), now);
        due.sort();
        let mut expected = vec![a.clone(), b.clone()];
        expected.sort();
        assert_eq!(due, expected);

        // Há 2 min: só a de 1 min vence (a outra usa o padrão de 5 min).
        let last: HashMap<_, _> = [(a.clone(), now - Duration::from_secs(120)), (b.clone(), now - Duration::from_secs(120))].into();
        assert_eq!(due_accounts(&accounts, &settings, &last, now), vec![b.clone()]);
        // A do padrão, verificada há 5 min, vence; a de 1 min, verificada há 59 s, ainda não.
        let last: HashMap<_, _> = [(a.clone(), now - Duration::from_secs(300)), (b.clone(), now - Duration::from_secs(59))].into();
        assert_eq!(due_accounts(&accounts, &settings, &last, now), vec![a.clone()]);
        assert!(!due_accounts(&accounts, &settings, &last, now).contains(&m));
    }

    #[test]
    fn first_sync_and_uidvalidity_change_reset() {
        assert_eq!(plan(&SavedState::default(), &sel(1, 10, 9, None)), Plan::Reset);
        let saved = SavedState { uidvalidity: Some(1), uidnext: Some(10), highestmodseq: Some(5), oldest_uid: Some(1) };
        assert_eq!(plan(&saved, &sel(2, 10, 9, Some(5))), Plan::Reset);
    }

    #[test]
    fn incremental_plan_fetches_only_what_changed() {
        let saved = SavedState { uidvalidity: Some(1), uidnext: Some(100), highestmodseq: Some(50), oldest_uid: Some(10) };
        assert_eq!(plan(&saved, &sel(1, 100, 90, Some(50))), Plan::Incremental { oldest: 10, new_from: None, flags_since: FlagsSince::Nothing });
        assert_eq!(plan(&saved, &sel(1, 103, 93, Some(57))), Plan::Incremental { oldest: 10, new_from: Some(100), flags_since: FlagsSince::Modseq(50) });
        let no_condstore = SavedState { highestmodseq: None, ..saved.clone() };
        assert_eq!(plan(&no_condstore, &sel(1, 100, 90, None)), Plan::Incremental { oldest: 10, new_from: None, flags_since: FlagsSince::All });
        assert_eq!(plan(&saved, &sel(1, 100 + INITIAL_WINDOW + 1, 90, Some(51))), Plan::Reset, "chegou mais que uma janela inteira");
    }

    #[test]
    fn chunks_cover_the_range_exactly() {
        assert_eq!(chunks(1, 600, 250), vec![(1, 250), (251, 500), (501, 600)]);
        assert_eq!(chunks(5, 5, 250), vec![(5, 5)]);
        assert!(chunks(10, 9, 250).is_empty());
        assert_eq!(chunks(u32::MAX - 1, u32::MAX, 250), vec![(u32::MAX - 1, u32::MAX)]);
    }

    /// Primeira sincronização simulada de uma pasta com 5.000 mensagens: o banco fica com
    /// exatamente as 2.000 mais recentes e a pasta não é marcada como completa.
    #[test]
    fn first_sync_shows_200_and_backfills_in_batches_up_to_2000() {
        let exists = 5000;
        let (start, end) = initial_window(exists).unwrap();
        assert_eq!((start, end), (4801, 5000), "as 200 mais recentes primeiro");
        let headers: Vec<_> = (start..=end).map(|seq| header(seq * 10, &format!("<{seq}@x>"), "s", false)).collect();
        let snap = Snapshot {
            reset: true,
            uidvalidity: 1,
            uidnext: Some(exists * 10 + 1),
            total: exists,
            oldest_uid: headers.iter().map(|h| h.uid).min(),
            complete: Some(false),
            headers,
            ..Default::default()
        };
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            store::apply_snapshot(c, inbox.id, &snap).unwrap();
            let f = store::get_folder(c, inbox.id).unwrap().unwrap();
            assert_eq!(backfill_batch(store::stored_in_folder(c, inbox.id).unwrap(), &f), Some(250));
            assert_eq!(backfill_batch(1900, &f), Some(100), "o último pacote só completa a janela");
            assert_eq!(backfill_batch(2000, &f), None);
            assert_eq!(backfill_batch(10, &store::FolderRow { complete: true, ..f.clone() }), None, "pasta pequena já inteira");
            assert_eq!(backfill_batch(0, &store::FolderRow { oldest_uid: None, ..f }), None, "nunca sincronizada");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn initial_sync_of_5000_messages_keeps_the_2000_most_recent() {
        let exists = 5000;
        // A janela inteira, depois de todos os pacotes da primeira sincronização.
        let (start, end) = imap_client::page_range(exists, INITIAL_WINDOW).unwrap();
        // Servidor simulado: sequência N tem UID 10 * N.
        let headers: Vec<_> = chunks(start, end, FETCH_CHUNK)
            .into_iter()
            .flat_map(|(a, b)| a..=b)
            .map(|seq| header(seq * 10, &format!("<{seq}@x>"), "s", seq % 2 == 0))
            .collect();
        let snap = Snapshot {
            reset: true,
            uidvalidity: 1,
            uidnext: Some(exists * 10 + 1),
            total: exists,
            unread: Some(2500),
            oldest_uid: headers.iter().map(|h| h.uid).min(),
            complete: Some(start == 1),
            headers,
            ..Default::default()
        };
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            store::apply_snapshot(c, inbox.id, &snap).unwrap();
            let stored: i64 = c.query_row("SELECT count(*) FROM message_folder WHERE folder_id = ?1", [inbox.id], |r| r.get(0)).unwrap();
            assert_eq!(stored, 2000);
            let f = store::get_folder(c, inbox.id).unwrap().unwrap();
            assert_eq!((f.oldest_uid, f.complete, f.total), (Some(30010), false, Some(5000)));
            let newest = store::list_page(c, inbox.id, None, 1).unwrap();
            assert_eq!(newest[0].uid, 50000);
            assert_eq!(newest[0].flags, MessageFlags { seen: true, flagged: false });
            Ok(())
        })
        .unwrap();
    }
}
