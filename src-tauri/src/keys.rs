//! Chave do banco (US10, CA2 e CA4): escolhe, ao abrir o app, a chave que abre o arquivo, troca a
//! chave sem perder dados e liga ou desliga a senha do Epistula. A ordem da troca garante que o
//! cofre sempre tenha uma chave que abre o banco: a nova vai primeiro para `KeySlot::Next`, o banco é
//! cifrado de novo numa transação e só então ela vira a atual. Se o app fechar no meio,
//! `open_database` conclui ou desfaz a troca. Logs: só etapas.

use std::path::Path;

use crate::db::{self, Db, DbKey, OpenError};
use crate::error::AppError;
use crate::lock::{self, Kek};
use crate::secrets::{self, KeySlot, StoredKey};

/// Onde a chave do banco fica guardada: o cofre do sistema (em teste, a memória).
pub trait KeyVault {
    fn load(&self, slot: KeySlot) -> Result<Option<StoredKey>, AppError>;
    fn save(&self, slot: KeySlot, key: &StoredKey) -> Result<(), AppError>;
    fn delete(&self, slot: KeySlot) -> Result<(), AppError>;
}

/// O Cofre de Credenciais do sistema.
pub struct Keyring;

impl KeyVault for Keyring {
    fn load(&self, slot: KeySlot) -> Result<Option<StoredKey>, AppError> {
        secrets::load_db_key(slot)
    }

    fn save(&self, slot: KeySlot, key: &StoredKey) -> Result<(), AppError> {
        secrets::save_db_key(slot, key)
    }

    fn delete(&self, slot: KeySlot) -> Result<(), AppError> {
        secrets::delete_db_key(slot)
    }
}

/// Como a chave do banco é guardada: crua (protegida só pelo cofre do sistema) ou envolvida pela
/// chave da senha do Epistula.
#[derive(Clone, Debug)]
pub enum Protection {
    Plain,
    Password(Kek),
}

impl Protection {
    pub fn store(&self, key: &DbKey) -> Result<StoredKey, AppError> {
        match self {
            Protection::Plain => Ok(StoredKey::Raw(key.clone())),
            Protection::Password(kek) => lock::wrap(kek, key).map(StoredKey::Wrapped),
        }
    }

    pub fn open(&self, stored: &StoredKey) -> Result<DbKey, AppError> {
        match (stored, self) {
            (StoredKey::Raw(key), _) => Ok(key.clone()),
            (StoredKey::Wrapped(_), Protection::Plain) => Err(AppError::Locked),
            (StoredKey::Wrapped(w), Protection::Password(kek)) => lock::unwrap(kek, w),
        }
    }

    pub fn has_password(&self) -> bool {
        matches!(self, Protection::Password(_))
    }
}

/// Banco aberto, com a chave e a proteção em uso.
pub struct Opened {
    pub db: Db,
    pub key: DbKey,
    pub protection: Protection,
}

fn opens(path: &Path, key: &DbKey) -> bool {
    path.exists() && db::opens_with(path, key)
}

/// Abre o banco com a chave do cofre. Com a senha ligada e sem `password`, devolve `Locked` antes
/// de tocar em qualquer coisa; com a senha errada, `WrongPassword`, sem pôr nada de lado. Sem chave,
/// ou com uma chave que não abre o arquivo, o arquivo é posto de lado e um banco novo é criado: o
/// banco é um cache do servidor (US3, CA1).
pub fn open_database(dir: &Path, vault: &dyn KeyVault, password: Option<&str>) -> Result<Opened, AppError> {
    let current = vault.load(KeySlot::Current)?;
    let protection = match (&current, password) {
        (Some(StoredKey::Wrapped(_)), None) => return Err(AppError::Locked),
        (Some(StoredKey::Wrapped(w)), Some(password)) => Protection::Password(lock::derive(password, w)?),
        _ => Protection::Plain,
    };
    // Confere a senha (tag do GCM) antes de qualquer outra coisa.
    let current = current.map(|s| protection.open(&s)).transpose()?;
    std::fs::create_dir_all(dir).map_err(|e| {
        tracing::error!(error = %e, "não foi possível criar a pasta de dados");
        AppError::Storage
    })?;
    let path = dir.join(db::FILE_NAME);
    // Uma chave próxima que não abre com a proteção atual é sobra sem uso.
    let next = vault.load(KeySlot::Next)?.and_then(|s| protection.open(&s).ok());
    if let Some(next) = next {
        let current_opens = current.as_ref().is_some_and(|k| opens(&path, k));
        if !current_opens && opens(&path, &next) {
            // A troca parou depois de cifrar o banco de novo: a chave nova passa a ser a atual.
            vault.save(KeySlot::Current, &protection.store(&next)?)?;
            vault.delete(KeySlot::Next)?;
            tracing::info!("troca de chave interrompida foi concluída");
            let db = Db::open(&path, &next).map_err(|_| AppError::Storage)?;
            return Ok(Opened { db, key: next, protection });
        }
        // A troca parou antes de mudar o banco: a chave nova não serve para nada.
        vault.delete(KeySlot::Next)?;
        tracing::info!("troca de chave interrompida foi desfeita");
    }
    let key = match current {
        Some(key) => key,
        None => {
            if path.exists() {
                tracing::warn!("chave do banco ausente no keyring");
                db::set_aside(&path);
            }
            let key = db::new_key();
            vault.save(KeySlot::Current, &protection.store(&key)?)?;
            key
        }
    };
    let db = match Db::open(&path, &key) {
        Ok(db) => db,
        Err(OpenError::WrongKey) => {
            tracing::warn!("a chave do keyring não abre o banco");
            db::set_aside(&path);
            Db::open(&path, &key).map_err(|_| AppError::Storage)?
        }
        Err(OpenError::Other) => return Err(AppError::Storage),
    };
    Ok(Opened { db, key, protection })
}

/// Troca a chave do banco aberto em `path` por uma nova, sem perder dados. Devolve a chave nova.
pub async fn rotate(db: &Db, path: &Path, vault: &(dyn KeyVault + Sync), protection: &Protection) -> Result<DbKey, AppError> {
    let new = db::new_key();
    vault.save(KeySlot::Next, &protection.store(&new)?)?;
    let key = new.clone();
    if let Err(e) = db.call(move |c| Db::rekey(c, &key)).await {
        // O rekey é uma transação: o banco continua com a chave antiga.
        let _ = vault.delete(KeySlot::Next);
        return Err(e);
    }
    if !opens(path, &new) {
        // A chave nova continua em `Next`; a próxima abertura resolve qual das duas abre.
        tracing::error!("o banco não abriu com a chave nova numa conexão separada");
        return Err(AppError::Storage);
    }
    vault.save(KeySlot::Current, &protection.store(&new)?)?;
    // Se falhar, a próxima abertura apaga a sobra (a atual já abre).
    let _ = vault.delete(KeySlot::Next);
    tracing::info!("chave do banco trocada");
    Ok(new)
}

/// Guarda a chave atual com outra proteção (ligar, trocar ou desligar a senha), sem recriar o banco.
pub fn protect(vault: &dyn KeyVault, key: &DbKey, protection: &Protection) -> Result<(), AppError> {
    let stored = protection.store(key)?;
    vault.save(KeySlot::Current, &stored)?;
    // Confere lendo de volta e abrindo com a proteção nova.
    match vault.load(KeySlot::Current)? {
        Some(s) if protection.open(&s).as_ref() == Ok(key) => Ok(()),
        _ => {
            tracing::error!("a chave guardada com a proteção nova não confere");
            Err(AppError::Keyring)
        }
    }
}

/// A senha confere com a chave guardada? Usado antes de trocar ou desligar a senha.
pub fn check_password(vault: &dyn KeyVault, password: &str) -> Result<Kek, AppError> {
    match vault.load(KeySlot::Current)? {
        Some(StoredKey::Wrapped(w)) => {
            let kek = lock::derive(password, &w)?;
            lock::unwrap(&kek, &w)?;
            Ok(kek)
        }
        _ => Err(AppError::InvalidInput),
    }
}

/// "Esqueci a senha": apaga o banco e as chaves e abre um banco novo, sem senha.
pub fn wipe(dir: &Path, vault: &dyn KeyVault) -> Result<Opened, AppError> {
    let path = dir.join(db::FILE_NAME);
    for side in ["", "-wal", "-shm"] {
        let file = std::path::PathBuf::from(format!("{}{side}", path.display()));
        if file.exists() {
            std::fs::remove_file(&file).map_err(|e| {
                tracing::error!(error = %e, "não foi possível apagar o banco");
                AppError::Storage
            })?;
        }
    }
    vault.delete(KeySlot::Next)?;
    vault.delete(KeySlot::Current)?;
    tracing::info!("dados locais apagados a pedido do usuário");
    open_database(dir, vault, None)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct Memory {
        keys: Mutex<HashMap<KeySlot, StoredKey>>,
        fail_save: Mutex<Option<KeySlot>>,
    }

    impl Memory {
        fn get(&self, slot: KeySlot) -> Option<StoredKey> {
            self.keys.lock().unwrap().get(&slot).cloned()
        }

        fn raw(&self, slot: KeySlot) -> Option<DbKey> {
            match self.get(slot) {
                Some(StoredKey::Raw(k)) => Some(k),
                _ => None,
            }
        }
    }

    impl KeyVault for Memory {
        fn load(&self, slot: KeySlot) -> Result<Option<StoredKey>, AppError> {
            Ok(self.get(slot))
        }
        fn save(&self, slot: KeySlot, key: &StoredKey) -> Result<(), AppError> {
            if *self.fail_save.lock().unwrap() == Some(slot) {
                return Err(AppError::Keyring);
            }
            self.keys.lock().unwrap().insert(slot, key.clone());
            Ok(())
        }
        fn delete(&self, slot: KeySlot) -> Result<(), AppError> {
            self.keys.lock().unwrap().remove(&slot);
            Ok(())
        }
    }

    const COST: lock::Cost = lock::Cost { m_kib: 19 * 1024, t: 1, p: 1 };

    fn mark(db: &Db) {
        db.with(|c| c.execute_batch("INSERT INTO app_setting (key, value) VALUES ('marca', 'x')").map_err(db::sql_error("teste"))).unwrap();
    }

    fn has_mark(db: &Db) -> bool {
        db.with(|c| Ok(c.query_row("SELECT count(*) FROM app_setting WHERE key = 'marca'", [], |r| r.get::<_, i64>(0)).unwrap()))
            .unwrap()
            == 1
    }

    fn set_aside_files(dir: &Path) -> usize {
        std::fs::read_dir(dir).unwrap().filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().contains("unreadable")).count()
    }

    fn open(dir: &Path, vault: &Memory) -> Opened {
        open_database(dir, vault, None).unwrap()
    }

    #[test]
    fn first_open_creates_and_saves_a_key() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Memory::default();
        let db = open(dir.path(), &vault).db;
        mark(&db);
        drop(db);
        assert!(vault.raw(KeySlot::Current).is_some());
        assert!(has_mark(&open(dir.path(), &vault).db));
    }

    #[tokio::test]
    async fn rotate_keeps_data_and_leaves_only_the_new_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(db::FILE_NAME);
        let vault = Memory::default();
        let db = open(dir.path(), &vault).db;
        mark(&db);
        let old = vault.raw(KeySlot::Current).unwrap();
        let new = rotate(&db, &path, &vault, &Protection::Plain).await.unwrap();
        drop(db);
        assert_eq!(vault.raw(KeySlot::Current), Some(new.clone()));
        assert_ne!(new, old);
        assert!(vault.get(KeySlot::Next).is_none());
        assert!(!db::opens_with(&path, &old), "a chave antiga não abre mais");
        assert!(has_mark(&open(dir.path(), &vault).db));
        assert_eq!(set_aside_files(dir.path()), 0, "nada foi posto de lado");
    }

    #[test]
    fn interrupted_after_rekey_is_completed_on_open() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Memory::default();
        let db = open(dir.path(), &vault).db;
        mark(&db);
        // O app fechou depois do rekey e antes de gravar a chave nova como atual.
        let new = db::new_key();
        vault.save(KeySlot::Next, &StoredKey::Raw(new.clone())).unwrap();
        db.with(|c| Db::rekey(c, &new)).unwrap();
        drop(db);
        let db = open(dir.path(), &vault).db;
        assert!(has_mark(&db));
        assert_eq!(vault.raw(KeySlot::Current), Some(new));
        assert!(vault.get(KeySlot::Next).is_none());
        assert_eq!(set_aside_files(dir.path()), 0);
    }

    #[test]
    fn interrupted_before_rekey_is_undone_on_open() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Memory::default();
        let db = open(dir.path(), &vault).db;
        mark(&db);
        drop(db);
        let old = vault.raw(KeySlot::Current).unwrap();
        vault.save(KeySlot::Next, &StoredKey::Raw(db::new_key())).unwrap();
        let db = open(dir.path(), &vault).db;
        assert!(has_mark(&db));
        assert_eq!(vault.raw(KeySlot::Current), Some(old));
        assert!(vault.get(KeySlot::Next).is_none());
    }

    #[tokio::test]
    async fn failure_saving_the_new_key_never_loses_the_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(db::FILE_NAME);
        let vault = Memory::default();
        let db = open(dir.path(), &vault).db;
        mark(&db);
        *vault.fail_save.lock().unwrap() = Some(KeySlot::Current);
        assert!(rotate(&db, &path, &vault, &Protection::Plain).await.is_err());
        drop(db);
        *vault.fail_save.lock().unwrap() = None;
        let db = open(dir.path(), &vault).db;
        assert!(has_mark(&db), "a abertura usa a chave nova guardada como próxima");
        assert_eq!(set_aside_files(dir.path()), 0);
    }

    #[tokio::test]
    async fn failure_saving_the_next_key_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(db::FILE_NAME);
        let vault = Memory::default();
        let db = open(dir.path(), &vault).db;
        mark(&db);
        let old = vault.raw(KeySlot::Current).unwrap();
        *vault.fail_save.lock().unwrap() = Some(KeySlot::Next);
        assert!(rotate(&db, &path, &vault, &Protection::Plain).await.is_err());
        drop(db);
        assert!(db::opens_with(&path, &old));
        *vault.fail_save.lock().unwrap() = None;
        assert!(has_mark(&open(dir.path(), &vault).db));
    }

    #[tokio::test]
    async fn password_protects_the_key_and_never_sets_the_database_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(db::FILE_NAME);
        let vault = Memory::default();
        let opened = open(dir.path(), &vault);
        mark(&opened.db);
        let kek = lock::new_kek_with("frase longa e secreta", COST).unwrap();
        let protection = Protection::Password(kek);
        protect(&vault, &opened.key, &protection).unwrap();
        assert!(matches!(vault.get(KeySlot::Current), Some(StoredKey::Wrapped(_))), "a chave crua saiu do cofre");
        // Trocar a chave com a senha ligada mantém a chave envolvida.
        rotate(&opened.db, &path, &vault, &protection).await.unwrap();
        assert!(matches!(vault.get(KeySlot::Current), Some(StoredKey::Wrapped(_))));
        drop(opened);

        assert!(matches!(open_database(dir.path(), &vault, None), Err(AppError::Locked)));
        assert!(matches!(open_database(dir.path(), &vault, Some("senha errada!")), Err(AppError::WrongPassword)));
        assert_eq!(set_aside_files(dir.path()), 0, "senha errada nunca põe o banco de lado");
        let reopened = open_database(dir.path(), &vault, Some("frase longa e secreta")).unwrap();
        assert!(has_mark(&reopened.db));
        assert!(reopened.protection.has_password());

        // Desligar a senha volta a chave crua, sem recriar o banco.
        let kek = check_password(&vault, "frase longa e secreta").unwrap();
        assert!(check_password(&vault, "senha errada!").is_err());
        drop(kek);
        protect(&vault, &reopened.key, &Protection::Plain).unwrap();
        drop(reopened);
        assert!(has_mark(&open(dir.path(), &vault).db));
    }

    #[test]
    fn wipe_starts_over_without_password() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Memory::default();
        let opened = open(dir.path(), &vault);
        mark(&opened.db);
        protect(&vault, &opened.key, &Protection::Password(lock::new_kek_with("frase longa e secreta", COST).unwrap())).unwrap();
        drop(opened);
        let fresh = wipe(dir.path(), &vault).unwrap();
        assert!(!has_mark(&fresh.db));
        assert!(!fresh.protection.has_password());
        assert!(vault.raw(KeySlot::Current).is_some());
    }
}
