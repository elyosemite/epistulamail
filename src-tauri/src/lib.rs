mod commands;
mod compose;
mod db;
mod error;
mod imap_client;
mod mailbox;
mod message;
mod oauth;
mod provider;
mod providers;
mod sanitize;
mod secrets;
mod session;
mod store;
mod sync;

use std::path::Path;

use db::{Db, OpenError};
use error::AppError;
use session::AppState;
use store::AccountStatus;
use tauri::Manager;

/// Abre o banco com a chave do keyring. Sem chave, ou com uma chave que não abre o arquivo, o
/// arquivo é posto de lado e um banco novo é criado: o banco é um cache do servidor (CA1).
fn open_database(dir: &Path) -> Result<Db, AppError> {
    std::fs::create_dir_all(dir).map_err(|e| {
        tracing::error!(error = %e, "não foi possível criar a pasta de dados");
        AppError::Storage
    })?;
    let path = dir.join(db::FILE_NAME);
    let key = match secrets::load_db_key()? {
        Some(key) => key,
        None => {
            if path.exists() {
                tracing::warn!("chave do banco ausente no keyring");
                db::set_aside(&path);
            }
            let key = db::new_key();
            secrets::save_db_key(&key)?;
            key
        }
    };
    match Db::open(&path, &key) {
        Ok(db) => Ok(db),
        Err(OpenError::WrongKey) => {
            tracing::warn!("a chave do keyring não abre o banco");
            db::set_aside(&path);
            Db::open(&path, &key).map_err(|_| AppError::Storage)
        }
        Err(OpenError::Other) => Err(AppError::Storage),
    }
}

/// Contas do índice do keyring que não estão no banco (banco recriado) voltam com nome e cor
/// padrão; ficam ativas se o refresh token ainda estiver no keyring.
async fn restore_from_index(state: &AppState) -> Result<(), AppError> {
    for entry in secrets::load_index()? {
        let Some(catalog) = providers::find(&entry.provider) else { continue };
        if !message::valid_account_id(&entry.id) {
            continue;
        }
        let has_token = secrets::load_refresh_token(&entry.id)?.is_some();
        let restored = state
            .db
            .call(move |c| {
                if store::get_account(c, &entry.id)?.is_some() {
                    return Ok(false);
                }
                let color = store::next_color(c)?;
                store::insert_account(c, &entry.id, catalog, &entry.email, &store::default_display_name(&entry.email), color)?;
                if !has_token {
                    store::set_status(c, &entry.id, AccountStatus::NeedsReauth)?;
                }
                Ok(true)
            })
            .await?;
        if restored {
            tracing::info!("conta recuperada do índice do keyring");
        }
    }
    Ok(())
}

/// A conta única da US1/US2 (`account` + `secret-N` no keyring) vira uma conta no banco. As
/// entradas antigas só são apagadas depois que as novas foram gravadas e conferidas (CA7).
async fn migrate_legacy_account(state: &AppState) -> Result<(), AppError> {
    let Some(legacy) = secrets::load_legacy()? else { return Ok(()) };
    let provider = providers::for_oauth(legacy.provider);
    let entry = providers::find(provider).ok_or(AppError::ProviderUnavailable)?;
    let email = legacy.email.clone();
    let existing = state.db.call(move |c| store::find_account(c, provider, &email)).await?;
    let id = match existing {
        Some(account) => account.id,
        None => {
            let id = store::new_account_id();
            let (new_id, email) = (id.clone(), legacy.email.clone());
            state
                .db
                .call(move |c| store::insert_account(c, &new_id, entry, &email, &store::default_display_name(&email), "blue"))
                .await?;
            id
        }
    };
    secrets::save_refresh_token(&id, &legacy.refresh_token)?;
    secrets::delete_legacy()?;
    tracing::info!("conta da versão anterior migrada para o banco");
    Ok(())
}

/// Contas de que o usuário saiu não podem ter segredo no keyring; uma remoção que falhou ao sair
/// é refeita aqui (CA6).
async fn clear_signed_out_secrets(state: &AppState) -> Result<(), AppError> {
    let accounts = state.db.call(|c| store::list_accounts(c)).await?;
    for account in accounts.iter().filter(|a| a.status == AccountStatus::SignedOut) {
        secrets::delete_refresh_token(&account.id)?;
    }
    Ok(())
}

async fn startup(state: &AppState) {
    for (step, result) in [
        ("índice", restore_from_index(state).await),
        ("migração", migrate_legacy_account(state).await),
        ("limpeza", clear_signed_out_secrets(state).await),
    ] {
        if let Err(e) = result {
            tracing::error!(step, kind = e.kind(), "etapa de inicialização falhou");
        }
    }
    state.save_index().await;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if cfg!(debug_assertions) {
        let _ = dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../.env"));
    }
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).with_target(false).init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let db = open_database(&dir).map_err(|e| {
                tracing::error!(kind = e.kind(), "não foi possível abrir o banco local");
                Box::new(e) as Box<dyn std::error::Error>
            })?;
            let state = AppState::new(app.handle().clone(), db)?;
            app.manage(state);
            let state = app.state::<AppState>();
            tauri::async_runtime::block_on(startup(&state));
            sync::start_periodic(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::list_providers,
            commands::connect_account,
            commands::reconnect_account,
            commands::cancel_connect,
            commands::update_account,
            commands::move_account,
            commands::sign_out_account,
            commands::set_last_folder,
            commands::list_folders,
            commands::list_messages,
            commands::sync_folder,
            commands::get_message,
            commands::set_flags,
            commands::open_external_link,
            commands::search_messages,
            commands::move_messages,
            commands::delete_messages,
            commands::archive_messages,
            commands::create_folder,
            commands::recent_folders,
            commands::set_followup,
            commands::compose_template,
            commands::get_draft,
            commands::save_draft,
            commands::delete_draft,
            commands::set_list_width,
        ])
        .build(tauri::generate_context!());

    let app = match app {
        Ok(app) => app,
        Err(e) => {
            tracing::error!(error = %e, "erro ao iniciar o Epistula");
            std::process::exit(1);
        }
    };

    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            if let Some(state) = handle.try_state::<AppState>() {
                tauri::async_runtime::block_on(state.close_all());
            }
        }
    });
}
