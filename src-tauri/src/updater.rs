//! Atualização automática (tauri-plugin-updater). O app pergunta pelo `latest.json` da última release
//! publicada no GitHub, ao abrir e a cada 6 horas; com uma versão nova, avisa uma vez no sino. O clique abre
//! a confirmação no frontend, e só então o instalador é baixado, conferido com a chave pública do
//! tauri.conf.json (um arquivo sem a assinatura certa é recusado) e executado; o app fecha e o instalador o
//! abre de novo. No build de desenvolvimento não há verificação.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::AppError;
use crate::session::AppState;
use crate::{notify, store};

const FIRST_CHECK: Duration = Duration::from_secs(20);
const EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// Última versão já avisada no sino, para não repetir o aviso a cada verificação.
const NOTIFIED_KEY: &str = "update_notified_version";

/// A versão nova encontrada na última verificação, pronta para instalar.
#[derive(Default)]
pub struct Updates {
    pending: Mutex<Option<Update>>,
}

impl Updates {
    fn get(&self) -> Option<Update> {
        self.pending.lock().ok().and_then(|p| p.clone())
    }

    fn set(&self, update: Option<Update>) {
        if let Ok(mut p) = self.pending.lock() {
            *p = update;
        }
    }
}

/// O que a confirmação mostra.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current: String,
    /// As notas da release (o trecho do CHANGELOG), em Markdown.
    pub notes: String,
}

fn info(update: &Update) -> UpdateInfo {
    UpdateInfo { version: update.version.clone(), current: update.current_version.clone(), notes: update.body.clone().unwrap_or_default() }
}

fn failed(what: &'static str) -> impl Fn(tauri_plugin_updater::Error) -> AppError {
    move |e| {
        tracing::warn!(error = %e, "{what}");
        AppError::UpdateFailed
    }
}

async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, AppError> {
    let updater = app.updater().map_err(failed("atualizador indisponível"))?;
    let found = updater.check().await.map_err(failed("verificação de atualização falhou"))?;
    let result = found.as_ref().map(info);
    if let Some(u) = &result {
        tracing::info!(version = %u.version, "versão nova disponível");
    }
    app.state::<Updates>().set(found);
    Ok(result)
}

/// Avisa no sino uma vez por versão. Com o app bloqueado (banco fechado), o aviso fica para a próxima vez.
async fn notify_once(app: &AppHandle, version: &str) {
    let state = app.state::<AppState>();
    let v = version.to_owned();
    let new = state
        .db
        .call(move |c| {
            if store::app_setting(c, NOTIFIED_KEY)?.as_deref() == Some(v.as_str()) {
                return Ok(false);
            }
            store::set_app_setting(c, NOTIFIED_KEY, &v)?;
            Ok(true)
        })
        .await;
    if matches!(new, Ok(true)) {
        notify::update_available(app, version);
    }
}

/// Verificação em segundo plano (só no app instalado).
pub fn start(app: &AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            if let Ok(Some(found)) = check(&app).await {
                notify_once(&app, &found.version).await;
            }
            tokio::time::sleep(EVERY).await;
        }
    });
}

/// A versão nova, se houver (verifica agora quando ainda não há uma guardada).
#[tauri::command]
pub async fn update_status(app: AppHandle) -> Result<Option<UpdateInfo>, AppError> {
    if cfg!(debug_assertions) {
        return Ok(None);
    }
    if let Some(update) = app.state::<Updates>().get() {
        return Ok(Some(info(&update)));
    }
    check(&app).await
}

/// Baixa, confere a assinatura e instala. O banco só fecha depois do download: se a rede cair no meio, o app
/// segue funcionando. No Windows o instalador assume e o app fecha; ele abre de novo ao terminar.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), AppError> {
    let update = match app.state::<Updates>().get() {
        Some(u) => u,
        None => {
            check(&app).await?;
            app.state::<Updates>().get().ok_or(AppError::UpdateFailed)?
        }
    };
    tracing::info!(version = %update.version, "baixando a atualização");
    let bytes = update.download(|_, _| {}, || {}).await.map_err(failed("download da atualização falhou"))?;
    app.state::<AppState>().close_all().await;
    update.install(bytes).map_err(failed("instalação da atualização falhou"))?;
    app.restart();
}
