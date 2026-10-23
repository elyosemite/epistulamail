use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::AppError;
use crate::imap_client::{self, MessageHeader};
use crate::oauth;
use crate::provider::Provider;
use crate::session::{Account, AppState};

#[tauri::command]
pub async fn get_account(state: State<'_, AppState>) -> Result<Option<Account>, AppError> {
    state.account().await
}

#[tauri::command]
pub async fn connect_account(app: AppHandle, state: State<'_, AppState>, provider: Provider) -> Result<Account, AppError> {
    let cancel = state.begin_connect();
    let result = oauth::authorize(&state.http, provider, cancel, |url| {
        app.opener().open_url(url, None::<&str>).map_err(|e| {
            tracing::error!(error = %e, "não foi possível abrir o navegador do sistema");
            AppError::AuthFailed
        })
    })
    .await;
    state.end_connect();
    let (tokens, email) = result?;
    state.store_oauth_account(provider, email, tokens).await
}

#[tauri::command]
pub fn cancel_connect(state: State<'_, AppState>) {
    state.cancel_connect();
}

/// Lista a INBOX. Se o servidor recusar o token (revogado antes de expirar), renova uma vez e tenta de novo.
#[tauri::command]
pub async fn list_inbox(state: State<'_, AppState>) -> Result<Vec<MessageHeader>, AppError> {
    let (account, token) = state.access(false).await?;
    match imap_client::fetch_inbox(account.provider, &account.email, &token).await {
        Err(AppError::ImapAuth) => {
            let (account, token) = state.access(true).await?;
            imap_client::fetch_inbox(account.provider, &account.email, &token).await
        }
        other => other,
    }
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<(), AppError> {
    state.disconnect().await
}
