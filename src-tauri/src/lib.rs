mod commands;
mod compose;
mod db;
mod error;
mod imap_client;
mod keys;
mod links;
mod lock;
mod mailbox;
mod message;
mod oauth;
mod moves;
mod notify;
mod offline;
mod photo;
mod preview;
mod privacy;
mod provider;
mod providers;
mod sanitize;
mod send;
mod secrets;
mod session;
mod store;
mod sync;
mod tray;

use db::Db;
use error::AppError;
use session::AppState;
use store::AccountStatus;
use tauri::Manager;

/// Uma falha do app não grava um despejo de memória com e-mails e chaves para os relatórios de erro
/// do Windows (US10, CA5). Só o código do erro vai para o log.
#[cfg(windows)]
fn exclude_from_error_reports() {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::ErrorReporting::WerAddExcludedApplication;
    let Ok(exe) = std::env::current_exe() else { return };
    let wide: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: `wide` é um texto UTF-16 terminado em zero que vive até o fim da chamada.
    let hr = unsafe { WerAddExcludedApplication(wide.as_ptr(), 0) };
    if hr < 0 {
        tracing::warn!(code = hr, "não foi possível tirar o app dos relatórios de erro do Windows");
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
/// Início do processo, para medir quanto a primeira tela demora (`app_ready`).
pub static STARTED_AT: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

pub fn run() {
    STARTED_AT.get_or_init(std::time::Instant::now);
    if cfg!(debug_assertions) {
        let _ = dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../.env"));
    }
    // O leitor de HTML dos e-mails (html5ever, pelo ammonia) avisa de cada tabela mal formada, comum
    // em newsletters ("foster parenting not implemented"); isso não é problema do app e enchia o log.
    use tracing_subscriber::prelude::*;
    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(tracing::Level::INFO)
        .with_target("html5ever", tracing::Level::ERROR)
        .with_target("markup5ever", tracing::Level::ERROR);
    tracing_subscriber::registry().with(tracing_subscriber::fmt::layer().with_target(false)).with(filter).init();

    let app = tauri::Builder::default()
        // Abrir o app de novo (com ele na bandeja) só traz a janela que já existe (US8).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| notify::bring_to_front(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::Builder::new().args(["--minimized"]).build())
        .on_window_event(|window, event| {
            // Fechar a janela a esconde na bandeja, para as notificações continuarem (US8, CA5).
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle().clone();
                if app.state::<notify::Notifier>().settings().tray_on_close {
                    api.prevent_close();
                    let _ = window.hide();
                    tauri::async_runtime::spawn(async move {
                        let state = app.state::<AppState>();
                        if matches!(state.db.call(|c| store::take_tray_hint(c)).await, Ok(true)) {
                            notify::tray_hint(&app);
                        }
                    });
                }
            }
        })
        .setup(|app| {
            #[cfg(windows)]
            exclude_from_error_reports();
            let dir = app.path().app_data_dir()?;
            // Com a senha do Epistula ligada (US10), o banco só abre no desbloqueio.
            let opened = match keys::open_database(&dir, &keys::Keyring, None) {
                Ok(opened) => Some(opened),
                Err(AppError::Locked) => None,
                Err(e) => {
                    tracing::error!(kind = e.kind(), "não foi possível abrir o banco local");
                    return Err(Box::new(e));
                }
            };
            let state = AppState::new(app.handle().clone(), Db::closed())?;
            match opened {
                Some(opened) => privacy::install(&state, opened),
                None => {
                    state.lock.start_locked();
                    tracing::info!("app aberto bloqueado");
                }
            }
            app.manage(state);
            // A janela não espera a inicialização: o keyring e a limpeza rodam em segundo plano, e só
            // `list_accounts` aguarda o fim (a tela de carregamento aparece antes).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                state.wait_open().await;
                let started = std::time::Instant::now();
                startup(&state).await;
                state.mark_started();
                moves::resume(&handle).await;
                tracing::info!(ms = started.elapsed().as_millis() as u64, "inicialização concluída");
            });
            sync::start_periodic(app.handle());
            app.manage(notify::Notifier::default());
            app.manage(offline::Downloader::default());
            offline::start(app.handle());
            privacy::watch_windows_lock(app.handle());
            notify::start(app.handle());
            tray::create(app.handle())?;
            // Aberto com o Windows (D8): fica só na bandeja. Senão, a janela aparece.
            if !std::env::args().any(|a| a == "--minimized") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::app_ready,
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
            commands::get_sync_settings,
            commands::flagged_counts,
            commands::list_flagged,
            commands::get_appearance,
            commands::set_appearance,
            commands::set_default_sync_interval,
            commands::set_account_sync_interval,
            commands::undo_move,
            commands::send_message,
            commands::cancel_send,
            commands::account_overview,
            commands::sync_account,
            commands::get_conversation,
            commands::get_notify_settings,
            commands::set_notify_settings,
            commands::list_notices,
            commands::mark_notices_seen,
            commands::set_snooze,
            commands::prefetch_messages,
            commands::get_offline_settings,
            commands::set_offline_days,
            commands::rotate_db_key,
            commands::lock_status,
            commands::unlock_app,
            commands::lock_app,
            commands::set_app_password,
            commands::change_app_password,
            commands::remove_app_password,
            commands::wipe_local_data,
            commands::get_privacy_settings,
            commands::set_lock_settings,
            commands::disk_encryption,
            commands::delete_account_data,
            commands::purge_outside_period,
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
