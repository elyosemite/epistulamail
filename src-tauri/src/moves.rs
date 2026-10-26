//! Mover, arquivar e mandar para a Lixeira com "Desfazer" (US7, D1). O movimento é registrado no
//! banco na hora (`store::create_pending_moves`): a mensagem some da origem e aparece no destino
//! para o frontend. O MOVE só vai ao servidor quando o prazo do "Desfazer" acaba, executado aqui,
//! inclusive depois de fechar e abrir o app. Nada de conteúdo vai para log.

use std::collections::BTreeMap;
use std::time::Duration;

use futures::FutureExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppError;
use crate::imap_client;
use crate::message::{FolderKey, MessageKey};
use crate::session::{AppState, Lane};
use crate::store::{self, PendingMove};
use crate::sync;

/// Prazo do "Desfazer" (o mesmo do toast no frontend).
pub const UNDO_SECS: i64 = 6;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MovesFinished {
    account_id: String,
    /// Ids (`MessageKey` da origem) que o servidor não moveu e voltaram para a pasta de origem.
    failed: Vec<String>,
    /// Pastas cujas listas mudaram (origem das que voltaram).
    folder_ids: Vec<String>,
}

/// Executa os movimentos vencidos logo depois do prazo.
pub fn schedule(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(UNDO_SECS as u64) + Duration::from_millis(200)).await;
        run_due(&app).await;
    });
}

/// Abertura do app: refaz os movimentos interrompidos e os que venceram com o app fechado.
pub async fn resume(app: &AppHandle) {
    let state = app.state::<AppState>();
    match state.db.call(|c| store::resume_moves(c)).await {
        Ok(n) if n > 0 => tracing::info!(count = n, "movimentos interrompidos retomados"),
        Ok(_) => {}
        Err(e) => tracing::warn!(kind = e.kind(), "não foi possível retomar os movimentos"),
    }
    run_due(app).await;
}

pub async fn run_due(app: &AppHandle) {
    let state = app.state::<AppState>();
    let due = match state.db.call(|c| store::take_due_moves(c, store::now())).await {
        Ok(due) => due,
        Err(e) => {
            tracing::warn!(kind = e.kind(), "não foi possível ler os movimentos pendentes");
            return;
        }
    };
    let mut groups: BTreeMap<(String, i64, i64), Vec<PendingMove>> = BTreeMap::new();
    for m in due {
        groups.entry((m.account_id.clone(), m.from_folder_id, m.to_folder_id)).or_default().push(m);
    }
    let mut failed: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for ((account_id, from, to), moves) in groups {
        let ids: Vec<i64> = moves.iter().map(|m| m.id).collect();
        // Arquivo local (US15): o mover acaba no banco, sem servidor.
        let lookup = account_id.clone();
        let local = state.db.call(move |c| store::require_account(c, &lookup)).await.is_ok_and(|a| a.status == store::AccountStatus::Local);
        if local {
            let done = ids.clone();
            if let Err(e) = state.db.call(move |c| crate::archive_store::finish_local_moves(c, &done)).await {
                tracing::warn!(kind = e.kind(), "mover no arquivo local não concluído");
            }
            continue;
        }
        let uids: Vec<u32> = moves.iter().map(|m| m.from_uid).collect();
        let outcome = execute(&state, &account_id, from, to, &uids).await;
        let ok = outcome.is_ok();
        if let Err(e) = &outcome {
            tracing::warn!(count = uids.len(), kind = e.kind(), "o servidor não moveu as mensagens; elas voltaram para a pasta");
        } else {
            tracing::info!(count = uids.len(), "mensagens movidas no servidor");
        }
        if let Err(e) = state.db.call(move |c| store::finish_moves(c, &ids, ok)).await {
            tracing::warn!(kind = e.kind(), "não foi possível registrar o resultado do mover");
        }
        if ok {
            sync::spawn_folder_sync(app, account_id.clone(), to);
        } else {
            let entry = failed.entry(account_id.clone()).or_default();
            entry.0.extend(moves.iter().map(|m| MessageKey { account: account_id.clone(), folder: from, uid: m.from_uid }.encode()));
            entry.1.push(FolderKey { account: account_id.clone(), folder: from }.encode());
            entry.1.push(FolderKey { account: account_id.clone(), folder: to }.encode());
        }
    }
    for (account_id, (ids, mut folder_ids)) in failed {
        let n = ids.len();
        let title = if n == 1 { "1 conversa não foi movida".to_owned() } else { format!("{n} conversas não foram movidas") };
        crate::notify::failure(app, &account_id, title, "O servidor recusou. Elas voltaram para a pasta de origem.".into(), String::new());
        folder_ids.sort();
        folder_ids.dedup();
        let _ = app.emit("moves-finished", MovesFinished { account_id, failed: ids, folder_ids });
    }
    state.emit_accounts_changed();
}

async fn execute(state: &AppState, account_id: &str, from: i64, to: i64, uids: &[u32]) -> Result<(), AppError> {
    let (source, target) = state
        .db
        .call(move |c| Ok((store::get_folder(c, from)?.ok_or(AppError::FolderNotFound)?, store::get_folder(c, to)?.ok_or(AppError::FolderNotFound)?)))
        .await?;
    let uidvalidity = source.uidvalidity.ok_or(AppError::MessageNotFound)?;
    let handle = state.handle(account_id).await?;
    let (mailbox, target_mailbox) = (source.imap_name.clone(), target.imap_name.clone());
    imap_client::with_session(state, &handle, Lane::Interactive, |s| {
        let (mailbox, target_mailbox, wanted) = (mailbox.clone(), target_mailbox.clone(), uids.to_vec());
        async move {
            imap_client::select_checked(s, &mailbox, uidvalidity).await?;
            imap_client::move_uids(s, &wanted, &target_mailbox).await
        }
        .boxed()
    })
    .await
}
