use futures::FutureExt;
use reqwest::Url;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::AppError;
use crate::imap_client::{self, FlagResult, MessagePage};
use crate::mailbox::Folder;
use crate::message::{self, Cursor, MessageRef, MessageView};
use crate::oauth;
use crate::provider::Provider;
use crate::session::{Account, AppState};

const MAX_FLAG_IDS: usize = 500;
const MAX_URL_LEN: usize = 2048;

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

#[tauri::command]
pub async fn list_folders(state: State<'_, AppState>) -> Result<Vec<Folder>, AppError> {
    imap_client::with_session(&state, |s| imap_client::list_folders(s).boxed()).await
}

#[tauri::command]
pub async fn list_messages(state: State<'_, AppState>, folder_id: String, cursor: Option<String>) -> Result<MessagePage, AppError> {
    let mailbox = message::decode_folder_id(&folder_id)?;
    let cursor = cursor.as_deref().map(Cursor::decode).transpose()?;
    imap_client::with_session(&state, |s| {
        let mailbox = mailbox.clone();
        let cursor = cursor.clone();
        async move { imap_client::list_page(s, &mailbox, cursor).await }.boxed()
    })
    .await
}

#[tauri::command]
pub async fn get_message(state: State<'_, AppState>, message_id: String, allow_remote_images: bool) -> Result<MessageView, AppError> {
    let r = MessageRef::decode(&message_id)?;
    imap_client::with_session(&state, |s| imap_client::read_message(s, r.clone(), allow_remote_images).boxed()).await
}

#[tauri::command]
pub async fn set_flags(
    state: State<'_, AppState>,
    message_ids: Vec<String>,
    seen: Option<bool>,
    flagged: Option<bool>,
) -> Result<Vec<FlagResult>, AppError> {
    if message_ids.is_empty() || message_ids.len() > MAX_FLAG_IDS || (seen.is_none() && flagged.is_none()) {
        return Err(AppError::InvalidId);
    }
    let refs = message_ids.iter().map(|id| MessageRef::decode(id)).collect::<Result<Vec<_>, _>>()?;
    imap_client::with_session(&state, |s| imap_client::set_flags(s, refs.clone(), seen, flagged).boxed()).await
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

/// Abre um link do corpo da mensagem no navegador (ou cliente de e-mail) do sistema.
#[tauri::command]
pub fn open_external_link(app: AppHandle, url: String) -> Result<(), AppError> {
    let parsed = external_link(&url)?;
    tracing::info!(scheme = parsed.scheme(), "abrindo link externo");
    app.opener().open_url(parsed.as_str(), None::<&str>).map_err(|e| {
        tracing::warn!(error = %e, "não foi possível abrir o link");
        AppError::UnsupportedLink
    })
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<(), AppError> {
    state.disconnect().await
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
}
