//! Sincronização com o servidor (US3, CA3). O servidor é lido numa sessão IMAP só de
//! sincronização e o resultado é aplicado no banco numa transação (`store::apply_snapshot`).
//! O frontend lê sempre do banco e é avisado por eventos quando algo muda.

use std::time::Duration;

use futures::FutureExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppError;
use crate::imap_client::{self, ImapSession};
use crate::message::FolderKey;
use crate::session::{AccountHandle, AppState, Lane};
use crate::store::{self, AccountStatus, ApplyReport, FolderRow, Snapshot};

/// Primeira sincronização de uma pasta: as 2.000 mensagens mais recentes (D5).
pub const INITIAL_WINDOW: u32 = 2000;
/// Mensagens mais antigas buscadas por vez quando o usuário pagina além do que está salvo.
pub const OLDER_BATCH: u32 = 100;
const FETCH_CHUNK: u32 = 250;
pub const INBOX_INTERVAL: Duration = Duration::from_secs(5 * 60);
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

/// Intervalo de sequência da primeira sincronização (as `INITIAL_WINDOW` mais recentes).
pub fn initial_window(exists: u32) -> Option<(u32, u32)> {
    imap_client::page_range(exists, INITIAL_WINDOW)
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
    let snap = imap_client::with_session(state, handle, Lane::Sync, |s| {
        let mailbox = mailbox.clone();
        let saved = saved.clone();
        async move { remote_snapshot(s, &mailbox, &saved).await }.boxed()
    })
    .await?;
    let reset = snap.reset;
    let report = state.db.call(move |c| store::apply_snapshot(c, folder_id, &snap)).await?;
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

/// Traz do servidor o próximo bloco de mensagens mais antigas que as salvas. Devolve quantas vieram.
pub async fn fetch_older(state: &AppState, handle: &AccountHandle, folder: &FolderRow) -> Result<usize, AppError> {
    let (Some(oldest), Some(uidvalidity)) = (folder.oldest_uid, folder.uidvalidity) else { return Ok(0) };
    if folder.complete {
        return Ok(0);
    }
    let mailbox = folder.imap_name.clone();
    let headers = imap_client::with_session(state, handle, Lane::Interactive, |s| {
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
            let complete = uids.len() <= OLDER_BATCH as usize;
            let batch: Vec<u32> = uids.iter().rev().take(OLDER_BATCH as usize).copied().collect();
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
}

/// Sincroniza uma pasta (se não houver outra sincronização dela em andamento) e avisa o frontend.
pub async fn run_folder_sync(state: &AppState, account_id: &str, folder_id: i64) -> Result<(), AppError> {
    let handle = state.handle(account_id).await?;
    if !handle.try_begin_sync(folder_id) {
        return Ok(());
    }
    let result = sync_folder(state, &handle, folder_id).await.map(|_| ());
    handle.end_sync(folder_id);
    report_outcome(state, account_id, &result);
    if result.is_ok() {
        let folder_key = FolderKey { account: account_id.to_owned(), folder: folder_id }.encode();
        let _ = state.app.emit("mailbox-updated", MailboxUpdated { account_id: account_id.to_owned(), folder_id: folder_key });
        state.emit_accounts_changed();
    }
    result
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

/// A cada 5 minutos, sincroniza a Caixa de entrada de cada conta ativa (a primeira volta é na abertura).
pub fn start_periodic(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let state = app.state::<AppState>();
            let accounts = state.db.call(|c| store::list_accounts(c)).await.unwrap_or_default();
            for account in accounts.into_iter().filter(|a| a.status == AccountStatus::Active) {
                let _ = run_account_sync(&state, &account.id).await;
            }
            tokio::time::sleep(INBOX_INTERVAL).await;
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
    fn initial_sync_of_5000_messages_keeps_the_2000_most_recent() {
        let exists = 5000;
        let (start, end) = initial_window(exists).unwrap();
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
