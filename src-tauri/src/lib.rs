mod commands;
mod error;
mod imap_client;
mod keyring_store;
mod mailbox;
mod message;
mod oauth;
mod provider;
mod sanitize;
mod session;

use session::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if cfg!(debug_assertions) {
        let _ = dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../.env"));
    }
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).with_target(false).init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_account,
            commands::connect_account,
            commands::cancel_connect,
            commands::list_folders,
            commands::list_messages,
            commands::get_message,
            commands::set_flags,
            commands::open_external_link,
            commands::disconnect,
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
            let state = handle.state::<AppState>();
            tauri::async_runtime::block_on(imap_client::close_session(&state));
        }
    });
}
