mod commands;
mod error;
mod imap_client;
mod keyring_store;
mod oauth;
mod provider;
mod session;

use session::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if cfg!(debug_assertions) {
        let _ = dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../.env"));
    }
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).with_target(false).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_account,
            commands::connect_account,
            commands::cancel_connect,
            commands::list_inbox,
            commands::disconnect,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Epistula");
}
