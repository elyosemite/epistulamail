//! Privacidade e proteção local (US10): bloqueio e desbloqueio do app, senha do Epistula, troca da
//! chave do banco, dados guardados por conta e o aviso de disco sem BitLocker. A criptografia fica em
//! `lock.rs` e `keys.rs`; aqui fica a ordem das etapas e o estado do app. Logs: só eventos.

use std::path::PathBuf;
use std::time::Instant;
#[cfg(windows)]
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppError;
use crate::keys::{self, Keyring, Opened, Protection};
use crate::lock::{self, Attempts, Kek};
use crate::secrets::{self, SecretString};
use crate::session::AppState;
use crate::store;

/// De quanto em quanto tempo conferir se o sistema bloqueou a tela.
#[cfg(windows)]
const WINDOWS_LOCK_TICK: Duration = Duration::from_secs(3);

/// Chave e proteção do banco aberto, e as tentativas de senha (US10, CA2 e CA3).
#[derive(Default)]
pub struct LockState {
    inner: std::sync::Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    key: Option<crate::db::DbKey>,
    protection: Option<Protection>,
    /// A senha está ligada (vale também com o app bloqueado, quando não há `protection`).
    has_password: bool,
    attempts: Attempts,
}

impl LockState {
    fn with<T>(&self, f: impl FnOnce(&mut Inner) -> T) -> T {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut inner)
    }

    pub fn has_password(&self) -> bool {
        self.with(|i| i.has_password)
    }

    /// O app abriu bloqueado: a senha está ligada e nada foi aberto.
    pub fn start_locked(&self) {
        self.with(|i| i.has_password = true);
    }

    fn protection(&self) -> Option<Protection> {
        self.with(|i| i.protection.clone())
    }

    fn key(&self) -> Option<crate::db::DbKey> {
        self.with(|i| i.key.clone())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LockStatus {
    pub locked: bool,
    pub has_password: bool,
    /// Segundos até poder tentar a senha de novo.
    pub retry_in: u64,
}

pub fn status(state: &AppState) -> LockStatus {
    let retry_in = state.lock.with(|i| i.attempts.wait_seconds(Instant::now()));
    LockStatus { locked: !state.db.is_open(), has_password: state.lock.has_password(), retry_in }
}

pub fn data_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    app.path().app_data_dir().map_err(|_| AppError::Storage)
}

fn emit(state: &AppState) {
    let _ = state.app.emit("lock-changed", status(state));
    crate::tray::update_menu(&state.app);
}

/// Passa a usar um banco recém-aberto (na abertura do app, no desbloqueio ou depois de apagar tudo).
pub fn install(state: &AppState, opened: Opened) {
    let Opened { db, key, protection } = opened;
    secrets::set_seal(match &protection {
        Protection::Password(kek) => Some(kek.clone()),
        Protection::Plain => None,
    });
    state.lock.with(|i| {
        i.has_password = protection.has_password();
        i.key = Some(key);
        i.protection = Some(protection);
        i.attempts.succeeded();
    });
    state.db.attach(db);
    state.mark_open();
}

/// Desbloqueio (CA2): espera crescente entre tentativas erradas; senha errada não apaga nada.
pub async fn unlock(state: &AppState, password: SecretString) -> Result<(), AppError> {
    if state.db.is_open() {
        return Ok(());
    }
    if state.lock.with(|i| i.attempts.wait_seconds(Instant::now())) > 0 {
        return Err(AppError::UnlockWait);
    }
    let dir = data_dir(&state.app)?;
    let opened = tokio::task::spawn_blocking(move || keys::open_database(&dir, &Keyring, Some(&password)))
        .await
        .map_err(|_| AppError::Storage)?;
    match opened {
        Ok(opened) => {
            install(state, opened);
            tracing::info!("app desbloqueado");
            emit(state);
            Ok(())
        }
        Err(AppError::WrongPassword) => {
            state.lock.with(|i| i.attempts.failed(Instant::now()));
            tracing::info!("senha do app incorreta");
            emit(state);
            Err(AppError::WrongPassword)
        }
        Err(e) => Err(e),
    }
}

/// Bloqueio (CA3): encerra as conexões, apaga das memórias os tokens e a chave, fecha o banco e
/// limpa os dados de navegação da janela (D6). Sem senha, não há o que bloquear.
pub async fn lock_now(state: &AppState) -> Result<(), AppError> {
    if !state.lock.has_password() {
        return Err(AppError::InvalidInput);
    }
    if !state.db.is_open() {
        return Ok(());
    }
    state.forget_accounts().await;
    secrets::set_seal(None);
    state.lock.with(|i| {
        i.key = None;
        i.protection = None;
    });
    state.db.close();
    clear_browsing_data(&state.app);
    tracing::info!("app bloqueado");
    emit(state);
    Ok(())
}

/// Nada do e-mail aberto fica nos dados da janela (cache de imagens liberadas, D6).
pub fn clear_browsing_data(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(e) = window.clear_all_browsing_data() {
            tracing::warn!(error = %e, "dados de navegação não foram limpos");
        }
    }
}

/// Os refresh tokens de todas as contas, lidos com a chave atual e gravados com a nova.
fn reseal_tokens(from: Option<Kek>, to: Option<Kek>) -> Result<(), AppError> {
    let index = secrets::load_index()?;
    secrets::set_seal(from);
    let tokens: Vec<(String, SecretString)> = index
        .iter()
        .filter_map(|e| secrets::load_refresh_token(&e.id).ok().flatten().map(|t| (e.id.clone(), t)))
        .collect();
    secrets::set_seal(to);
    for (id, token) in &tokens {
        // Uma falha deixa só esta conta pedindo para reconectar; a chave do banco já está protegida.
        if let Err(e) = secrets::save_refresh_token(id, token) {
            tracing::warn!(kind = e.kind(), "refresh token não foi regravado com a proteção nova");
        }
    }
    Ok(())
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, AppError> + Send + 'static) -> Result<T, AppError> {
    tokio::task::spawn_blocking(f).await.map_err(|_| AppError::Storage)?
}

/// Liga a senha (CA2): a chave do banco e os tokens passam a ser guardados cifrados pela senha.
pub async fn set_password(state: &AppState, password: SecretString) -> Result<(), AppError> {
    let (Some(key), Some(Protection::Plain)) = (state.lock.key(), state.lock.protection()) else { return Err(AppError::InvalidInput) };
    let kek = blocking(move || lock::new_kek(&password)).await?;
    let protection = Protection::Password(kek.clone());
    let p = protection.clone();
    blocking(move || {
        keys::protect(&Keyring, &key, &p)?;
        reseal_tokens(None, Some(kek))
    })
    .await?;
    state.lock.with(|i| {
        i.protection = Some(protection);
        i.has_password = true;
    });
    tracing::info!("senha do app ligada");
    emit(state);
    Ok(())
}

/// Troca a senha: confere a atual e envolve a mesma chave com a nova.
pub async fn change_password(state: &AppState, current: SecretString, new: SecretString) -> Result<(), AppError> {
    let Some(key) = state.lock.key() else { return Err(AppError::Locked) };
    let (old, kek) = blocking(move || Ok((keys::check_password(&Keyring, &current)?, lock::new_kek(&new)?))).await?;
    let protection = Protection::Password(kek.clone());
    let p = protection.clone();
    blocking(move || {
        keys::protect(&Keyring, &key, &p)?;
        reseal_tokens(Some(old), Some(kek))
    })
    .await?;
    state.lock.with(|i| i.protection = Some(protection));
    tracing::info!("senha do app trocada");
    Ok(())
}

/// Desliga a senha: confere a atual e guarda a chave e os tokens só com o cofre do sistema.
pub async fn remove_password(state: &AppState, current: SecretString) -> Result<(), AppError> {
    let Some(key) = state.lock.key() else { return Err(AppError::Locked) };
    let old = blocking(move || keys::check_password(&Keyring, &current)).await?;
    blocking(move || {
        keys::protect(&Keyring, &key, &Protection::Plain)?;
        reseal_tokens(Some(old), None)
    })
    .await?;
    state.lock.with(|i| {
        i.protection = Some(Protection::Plain);
        i.has_password = false;
    });
    tracing::info!("senha do app desligada");
    emit(state);
    Ok(())
}

/// Troca a chave do banco (CA4), mantendo a proteção atual.
pub async fn rotate_key(state: &AppState) -> Result<(), AppError> {
    let Some(protection) = state.lock.protection() else { return Err(AppError::Locked) };
    let path = data_dir(&state.app)?.join(crate::db::FILE_NAME);
    let key = keys::rotate(&state.db, &path, &Keyring, &protection).await?;
    state.lock.with(|i| i.key = Some(key));
    Ok(())
}

/// "Esqueci a senha": apaga o banco, as chaves e os tokens (cifrados pela senha esquecida, não abrem
/// mais) e começa de novo sem senha. As contas voltam pelo índice pedindo para reconectar.
pub async fn wipe(state: &AppState) -> Result<(), AppError> {
    if state.db.is_open() {
        return Err(AppError::InvalidInput);
    }
    let dir = data_dir(&state.app)?;
    let opened = blocking(move || {
        for entry in secrets::load_index()? {
            secrets::delete_refresh_token(&entry.id)?;
        }
        keys::wipe(&dir, &Keyring)
    })
    .await?;
    install(state, opened);
    emit(state);
    Ok(())
}

/// Apaga do computador os dados de uma conta de que o usuário saiu (CA7) e troca a chave do banco,
/// para uma cópia antiga do arquivo não abrir com a chave atual (D4).
pub async fn delete_account_data(state: &AppState, account_id: String) -> Result<(), AppError> {
    let id = account_id.clone();
    state.db.call(move |c| store::delete_signed_out_account(c, &id)).await?;
    state.drop_handle(&account_id).await;
    let _ = secrets::delete_refresh_token(&account_id);
    state.save_index().await;
    compact_and_rotate(state).await;
    state.emit_accounts_changed();
    tracing::info!("dados de uma conta apagados deste computador");
    Ok(())
}

/// Depois de apagar dados: o arquivo encolhe e a chave é trocada. Uma falha aqui não desfaz a
/// exclusão (as páginas já foram zeradas pelo `secure_delete`).
pub async fn compact_and_rotate(state: &AppState) {
    if let Err(e) = state.db.call(|c| store::compact(c)).await {
        tracing::warn!(kind = e.kind(), "banco não foi compactado");
    }
    if let Err(e) = rotate_key(state).await {
        tracing::warn!(kind = e.kind(), "chave não foi trocada depois de apagar dados");
    }
}

/// Bloqueia junto com o Windows (CA3, D2), conferindo de tempos em tempos se a tela foi bloqueada.
/// No Linux, o bloqueio junto com a tela ainda não existe (US18); o bloqueio por inatividade continua.
#[cfg(windows)]
pub fn watch_system_lock(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut was_locked = false;
        loop {
            tokio::time::sleep(WINDOWS_LOCK_TICK).await;
            let locked = windows_session_locked();
            if locked && !was_locked {
                let state = app.state::<AppState>();
                if state.lock.has_password() && state.db.is_open() {
                    let follow = state.db.call(|c| store::lock_settings(c)).await.map(|s| s.with_windows).unwrap_or(false);
                    if follow {
                        let _ = lock_now(&state).await;
                    }
                }
            }
            was_locked = locked;
        }
    });
}

#[cfg(not(windows))]
pub fn watch_system_lock(_app: &AppHandle) {}

/// A tela do Windows está bloqueada? Bloqueada, a área de trabalho de entrada é a do Winlogon e não
/// pode ser aberta pelo app.
#[cfg(windows)]
fn windows_session_locked() -> bool {
    use windows_sys::Win32::System::StationsAndDesktops::{CloseDesktop, OpenInputDesktop, SwitchDesktop, DESKTOP_SWITCHDESKTOP};
    // SAFETY: chamadas sem ponteiros do app; o handle é fechado logo depois.
    unsafe {
        let desk = OpenInputDesktop(0, 0, DESKTOP_SWITCHDESKTOP);
        if desk.is_null() {
            return true;
        }
        let switched = SwitchDesktop(desk) != 0;
        CloseDesktop(desk);
        !switched
    }
}

/// O volume onde fica o banco está cifrado pelo BitLocker (CA6)? `None` quando não dá para saber
/// (edição do Windows sem BitLocker, outro sistema): sem aviso. Lê a propriedade que o Explorer
/// mostra, sem precisar de administrador.
pub async fn disk_encrypted(dir: PathBuf) -> Option<bool> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let root = dir.components().next().map(|c| c.as_os_str().to_string_lossy().to_string())?;
        if root.len() != 2 || !root.ends_with(':') || !root.chars().next()?.is_ascii_alphabetic() {
            return None;
        }
        let script = format!(
            "(New-Object -ComObject Shell.Application).NameSpace(17).ParseName('{root}').ExtendedProperty('System.Volume.BitLockerProtection')"
        );
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
        })
        .await
        .ok()?
        .ok()?;
        bitlocker_state(String::from_utf8_lossy(&output.stdout).trim())
    }
    #[cfg(target_os = "linux")]
    {
        let dir = dir.clone();
        return tokio::task::spawn_blocking(move || linux_disk_encrypted(&dir)).await.ok()?;
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = dir;
        None
    }
}

/// O volume onde fica o banco está cifrado com LUKS? `None` quando não dá para saber (sem `findmnt`
/// ou `lsblk`, ou saída inesperada).
#[cfg(target_os = "linux")]
fn linux_disk_encrypted(dir: &std::path::Path) -> Option<bool> {
    let dir = dir.to_string_lossy().to_string();
    let source = std::process::Command::new("findmnt").args(["-T", dir.as_str(), "-o", "SOURCE", "-n"]).output().ok()?;
    if !source.status.success() {
        return None;
    }
    let source = String::from_utf8_lossy(&source.stdout).trim().to_string();
    // Em btrfs, o findmnt devolve `/dev/sda2[/@home]`; o lsblk quer só o dispositivo.
    let source = source.split_once('[').map_or(source.as_str(), |(head, _)| head).to_string();
    if source.is_empty() {
        return None;
    }
    let stack = std::process::Command::new("lsblk").args(["-s", "-n", "-o", "TYPE", source.as_str()]).output().ok()?;
    if !stack.status.success() {
        return None;
    }
    Some(stack_has_crypt(&String::from_utf8_lossy(&stack.stdout)))
}

/// A pilha de dispositivos do `lsblk -s` tem um `crypt` (dm-crypt/LUKS)?
#[cfg(target_os = "linux")]
pub fn stack_has_crypt(output: &str) -> bool {
    output.lines().any(|line| line.trim() == "crypt")
}

/// Valores de `System.Volume.BitLockerProtection`: 1 ligado, 2 desligado, 3 cifrando, 4 decifrando,
/// 5 suspenso, 6 ligado e travado. Vazio: o volume não aceita BitLocker.
pub fn bitlocker_state(value: &str) -> Option<bool> {
    match value {
        "1" | "3" | "6" => Some(true),
        "2" | "4" | "5" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitlocker_values_map_to_a_warning_only_when_off() {
        assert_eq!(bitlocker_state("1"), Some(true));
        assert_eq!(bitlocker_state("2"), Some(false));
        assert_eq!(bitlocker_state("5"), Some(false), "suspenso não protege");
        assert_eq!(bitlocker_state(""), None, "sem BitLocker no volume: sem aviso");
        assert_eq!(bitlocker_state("lixo"), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn luks_stack_is_recognized() {
        assert!(stack_has_crypt("crypt\npart\ndisk\n"), "dm-crypt na pilha do lsblk");
        assert!(!stack_has_crypt("lvm\npart\ndisk\n"), "LVM sem LUKS não conta");
        assert!(!stack_has_crypt(""));
    }
}
