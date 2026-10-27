//! Bandeja do sistema e barra de tarefas (US8, CA5). Fechar a janela a esconde aqui; o menu do ícone
//! abre o app, começa um e-mail, sincroniza, silencia por 1 hora e sai de verdade. O tooltip mostra
//! as não lidas e o ícone da barra de tarefas ganha um ponto azul quando há alguma (D9). Com a senha
//! do Epistula ligada, o menu ganha "Bloquear o Epistula"; bloqueado, perde "Novo e-mail" e
//! "Sincronizar agora" (US10, CA3).

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::notify::{self, Notifier};
use crate::session::AppState;
use crate::{store, sync};

const TRAY_ID: &str = "epistula";
const SNOOZE_ID: &str = "snooze";

/// O item "Não perturbe" do menu, para o texto acompanhar o sino e o fim da 1 hora.
struct SnoozeItem(std::sync::Mutex<MenuItem<tauri::Wry>>);

/// Monta o menu conforme a senha e o bloqueio.
fn menu(app: &AppHandle, has_password: bool, locked: bool) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "open", "Abrir o EpistulaMail", true, None::<&str>)?;
    let compose = MenuItem::with_id(app, "compose", "Novo e-mail", true, None::<&str>)?;
    let sync_now = MenuItem::with_id(app, "sync", "Sincronizar agora", true, None::<&str>)?;
    let snooze = MenuItem::with_id(app, SNOOZE_ID, "Não perturbe por 1 hora", true, None::<&str>)?;
    let lock = MenuItem::with_id(app, "lock", "Bloquear o EpistulaMail", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair do EpistulaMail", true, None::<&str>)?;
    match app.try_state::<SnoozeItem>() {
        Some(item) => {
            if let Ok(mut current) = item.0.lock() {
                *current = snooze.clone();
            }
        }
        None => {
            app.manage(SnoozeItem(std::sync::Mutex::new(snooze.clone())));
        }
    }
    let (sep1, sep2) = (PredefinedMenuItem::separator(app)?, PredefinedMenuItem::separator(app)?);
    let mut items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = vec![&open];
    if !locked {
        items.extend([&compose as &dyn tauri::menu::IsMenuItem<tauri::Wry>, &sync_now]);
    }
    items.extend([&sep1 as &dyn tauri::menu::IsMenuItem<tauri::Wry>, &snooze, &sep2]);
    if has_password && !locked {
        items.push(&lock);
    }
    items.push(&quit);
    Menu::with_items(app, &items)
}

/// Refaz o menu depois de ligar ou desligar a senha, bloquear ou desbloquear.
pub fn update_menu(app: &AppHandle) {
    let state = app.state::<AppState>();
    let (has_password, locked) = (state.lock.has_password(), !state.db.is_open());
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), menu(app, has_password, locked)) {
        let _ = tray.set_menu(Some(menu));
        sync_snooze(app);
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let menu = menu(app, state.lock.has_password(), !state.db.is_open())?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID).tooltip("EpistulaMail").menu(&menu).show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open" => notify::bring_to_front(app),
            "compose" => {
                notify::bring_to_front(app);
                let _ = app.emit("tray-compose", ());
            }
            "sync" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<AppState>();
                    if let Ok(accounts) = state.db.call(|c| store::list_accounts(c)).await {
                        for a in accounts.into_iter().filter(|a| a.status == store::AccountStatus::Active) {
                            let _ = sync::run_account_sync(&state, &a.id).await;
                        }
                    }
                });
            }
            SNOOZE_ID => {
                let on = app.state::<Notifier>().toggle_snooze();
                sync_snooze(app);
                let _ = app.emit("notices-updated", ());
                tracing::info!(on, "não perturbe da bandeja");
            }
            "lock" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = crate::privacy::lock_now(&app.state::<AppState>()).await;
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                notify::bring_to_front(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Texto do item "Não perturbe" conforme o estado atual.
pub fn sync_snooze(app: &AppHandle) {
    let (Some(item), Some(n)) = (app.try_state::<SnoozeItem>(), app.try_state::<Notifier>()) else { return };
    let text = if n.snoozed() { "Retomar notificações" } else { "Não perturbe por 1 hora" };
    if let Ok(current) = item.0.lock() {
        let _ = current.set_text(text);
    };
}

/// Ponto azul de 16 px para o ícone sobreposto da barra de tarefas.
fn dot() -> Image<'static> {
    const SIZE: u32 = 16;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    let c = (SIZE as f32 - 1.0) / 2.0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let d = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
            let (alpha, ring) = (if d <= 7.5 { 255 } else { 0 }, d > 6.0);
            // Azul da marca com um aro branco para separar do ícone.
            let (r, g, b) = if ring { (255, 255, 255) } else { (0x0C, 0x66, 0xE4) };
            rgba.extend_from_slice(&[r, g, b, alpha]);
        }
    }
    Image::new_owned(rgba, SIZE, SIZE)
}

/// Atualiza o tooltip e o ponto conforme as não lidas da Caixa de entrada das contas ativas.
pub fn refresh(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let Ok(unread) = state.db.call(|c| store::inbox_unread_total(c)).await else { return };
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            let tip = match unread {
                0 => "EpistulaMail".to_owned(),
                1 => "EpistulaMail · 1 não lida".to_owned(),
                n => format!("EpistulaMail · {n} não lidas"),
            };
            let _ = tray.set_tooltip(Some(tip));
        }
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.set_overlay_icon((unread > 0).then(dot));
        }
    });
}
