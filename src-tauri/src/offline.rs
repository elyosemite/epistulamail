//! Download dos e-mails em segundo plano (US9, CA2 e CA3). Depois de cada sincronização, uma fila
//! de baixa prioridade baixa o corpo das mensagens ainda não salvas, em lotes pequenos e sem marcar
//! como lidas, para que abrir seja instantâneo e funcione sem conexão. Abrir uma mensagem usa outra
//! conexão (`Lane::Interactive`) e nunca espera a fila. Logs: só quantidades e tempos.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use chrono::Local;
use futures::FutureExt;
use tauri::{AppHandle, Manager};
use tokio::sync::Notify;

use crate::error::AppError;
use crate::imap_client;
use crate::message::{self, MessageFlags};
use crate::session::{AccountHandle, AppState, Lane};
use crate::store::{self, AccountStatus, BodyTarget};

/// Mensagens por lote (CA2).
pub const BATCH: usize = 25;
/// Maior mensagem baixada antes de abrir (D2); acima disso, quase sempre anexos grandes, só ao abrir.
pub const MAX_BACKGROUND_BYTES: u32 = 2 * 1024 * 1024;
/// Lotes por conta em cada volta, para uma conta grande não segurar as outras.
const ROUND_BATCHES: usize = 40;
/// Sem sincronização nova, a fila confere de novo depois desse tempo.
const IDLE: Duration = Duration::from_secs(120);
/// Pré-busca (CA3): no máximo estas mensagens por pedido.
pub const PREFETCH_LIMIT: usize = 10;

/// Acorda a fila quando uma sincronização termina.
#[derive(Default)]
pub struct Downloader {
    wake: Notify,
}

pub fn wake(app: &AppHandle) {
    if let Some(d) = app.try_state::<Downloader>() {
        d.wake.notify_one();
    }
}

pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        state.wait_ui_ready(Duration::from_secs(10)).await;
        state.wait_started().await;
        loop {
            round(&app).await;
            let Some(d) = app.try_state::<Downloader>() else { return };
            tokio::select! {
                _ = d.wake.notified() => {}
                _ = tokio::time::sleep(IDLE) => {}
            }
        }
    });
}

/// Uma volta por todas as contas ativas, até não sobrar nada no período escolhido.
async fn round(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Ok((accounts, choice)) = state.db.call(|c| Ok((store::list_accounts(c)?, store::offline_days(c)?))).await else { return };
    let Some(since) = store::offline_since(&choice, Local::now().date_naive()) else { return };
    for account in accounts.into_iter().filter(|a| a.status == AccountStatus::Active) {
        let Ok(handle) = state.handle(&account.id).await else { continue };
        let started = std::time::Instant::now();
        let mut tried: HashSet<i64> = HashSet::new();
        let mut saved = 0;
        for _ in 0..ROUND_BATCHES {
            let (id, since, skip) = (account.id.clone(), since.clone(), tried.clone());
            let Ok(targets) = state.db.call(move |c| store::bodies_to_download(c, &id, since.as_deref(), MAX_BACKGROUND_BYTES, BATCH, &skip)).await else { break };
            if targets.is_empty() {
                break;
            }
            tried.extend(targets.iter().map(|t| t.message_id));
            match download(&state, &handle, targets).await {
                Ok(n) => saved += n,
                // Sem conexão ou sem acesso: tenta de novo na próxima volta.
                Err(e) => {
                    tracing::info!(kind = e.kind(), "download em segundo plano pausado");
                    break;
                }
            }
        }
        if saved > 0 {
            tracing::info!(count = saved, ms = started.elapsed().as_millis() as u64, "e-mails baixados em segundo plano");
        }
    }
}

/// Baixa e salva os corpos de um lote (todas da mesma pasta). Devolve quantos foram salvos.
pub async fn download(state: &AppState, handle: &AccountHandle, targets: Vec<BodyTarget>) -> Result<usize, AppError> {
    let Some(folder_id) = targets.first().map(|t| t.folder_id) else { return Ok(0) };
    let folder = state.db.call(move |c| store::get_folder(c, folder_id)).await?.ok_or(AppError::InvalidId)?;
    let Some(uidvalidity) = folder.uidvalidity else { return Ok(0) };
    let uids: Vec<u32> = targets.iter().filter(|t| t.folder_id == folder_id).map(|t| t.uid).collect();
    let mailbox = folder.imap_name.clone();
    let fetched = imap_client::with_session(state, handle, Lane::Sync, |s| {
        let (mailbox, uids) = (mailbox.clone(), uids.clone());
        async move {
            imap_client::select_checked(s, &mailbox, uidvalidity).await?;
            imap_client::fetch_raw_many(s, &uids).await
        }
        .boxed()
    })
    .await?;
    let mut bodies: HashMap<u32, Vec<u8>> = fetched.into_iter().collect();
    state
        .db
        .call(move |c| {
            let mut saved = 0;
            for t in targets.iter().filter(|t| t.folder_id == folder_id) {
                let Some(raw) = bodies.remove(&t.uid) else { continue };
                // Os anexos vêm da mesma leitura que a tela usa; as flags não mudam aqui.
                let attachments = message::build_view(&raw, String::new(), MessageFlags::default(), None, false).attachments;
                store::save_body(c, t.message_id, &raw, &attachments)?;
                saved += 1;
            }
            Ok(saved)
        })
        .await
}

/// Pré-busca (CA3): as mensagens pedidas pelo frontend (as próximas da lista, as da conversa, a
/// linha sob o mouse), agrupadas por pasta, em segundo plano. Respeita "Só ao abrir".
pub fn prefetch(app: &AppHandle, account_id: String, keys: Vec<(i64, u32)>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let id = account_id.clone();
        let loaded = state
            .db
            .call(move |c| {
                let account = store::require_account(c, &id)?;
                let wanted = store::offline_days(c)? != "none" && account.status == AccountStatus::Active;
                Ok(if wanted { store::bodies_for_keys(c, &id, &keys, MAX_BACKGROUND_BYTES)? } else { Vec::new() })
            })
            .await;
        let Ok(targets) = loaded else { return };
        if targets.is_empty() {
            return;
        }
        let Ok(handle) = state.handle(&account_id).await else { return };
        let mut by_folder: HashMap<i64, Vec<BodyTarget>> = HashMap::new();
        for t in targets.into_iter().take(PREFETCH_LIMIT) {
            by_folder.entry(t.folder_id).or_default().push(t);
        }
        for (_, batch) in by_folder {
            if let Err(e) = download(&state, &handle, batch).await {
                tracing::info!(kind = e.kind(), "pré-busca ficou para depois");
                return;
            }
        }
    });
}
