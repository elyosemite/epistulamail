//! Chave do banco (US10, CA4): escolhe, ao abrir o app, a chave que abre o arquivo e troca a chave
//! sem perder dados. A ordem da troca garante que o cofre sempre tenha uma chave que abre o banco:
//! a nova vai primeiro para `KeySlot::Next`, o banco é cifrado de novo numa transação e só então ela
//! vira a atual. Se o app fechar no meio, `open_database` conclui ou desfaz a troca. Logs: só etapas.

use std::path::Path;

use crate::db::{self, Db, DbKey, OpenError};
use crate::error::AppError;
use crate::secrets::{self, KeySlot, StoredKey};

/// Onde a chave do banco fica guardada: o cofre do sistema (em teste, a memória).
pub trait KeyVault {
    fn load(&self, slot: KeySlot) -> Result<Option<DbKey>, AppError>;
    fn save(&self, slot: KeySlot, key: &DbKey) -> Result<(), AppError>;
    fn delete(&self, slot: KeySlot) -> Result<(), AppError>;
}

/// O Cofre de Credenciais do sistema.
pub struct Keyring;

impl KeyVault for Keyring {
    fn load(&self, slot: KeySlot) -> Result<Option<DbKey>, AppError> {
        match secrets::load_db_key(slot)? {
            Some(StoredKey::Raw(key)) => Ok(Some(key)),
            // A chave envolvida pela senha do Epistula só abre com o desbloqueio (US10, CA2).
            Some(StoredKey::Wrapped(_)) => {
                tracing::error!(?slot, "chave do banco protegida por senha e desbloqueio indisponível");
                Err(AppError::Keyring)
            }
            None => Ok(None),
        }
    }

    fn save(&self, slot: KeySlot, key: &DbKey) -> Result<(), AppError> {
        secrets::save_db_key(slot, &StoredKey::Raw(key.clone()))
    }

    fn delete(&self, slot: KeySlot) -> Result<(), AppError> {
        secrets::delete_db_key(slot)
    }
}

fn opens(path: &Path, key: &DbKey) -> bool {
    path.exists() && db::opens_with(path, key)
}

/// Abre o banco com a chave do cofre. Sem chave, ou com uma chave que não abre o arquivo, o
/// arquivo é posto de lado e um banco novo é criado: o banco é um cache do servidor (US3, CA1).
pub fn open_database(dir: &Path, vault: &dyn KeyVault) -> Result<Db, AppError> {
    std::fs::create_dir_all(dir).map_err(|e| {
        tracing::error!(error = %e, "não foi possível criar a pasta de dados");
        AppError::Storage
    })?;
    let path = dir.join(db::FILE_NAME);
    let current = vault.load(KeySlot::Current)?;
    if let Some(next) = vault.load(KeySlot::Next)? {
        let current_opens = current.as_ref().is_some_and(|k| opens(&path, k));
        if !current_opens && opens(&path, &next) {
            // A troca parou depois de cifrar o banco de novo: a chave nova passa a ser a atual.
            vault.save(KeySlot::Current, &next)?;
            vault.delete(KeySlot::Next)?;
            tracing::info!("troca de chave interrompida foi concluída");
            return Db::open(&path, &next).map_err(|_| AppError::Storage);
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
            vault.save(KeySlot::Current, &key)?;
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

/// Troca a chave do banco aberto em `path` por uma nova, sem perder dados.
pub async fn rotate(db: &Db, path: &Path, vault: &(dyn KeyVault + Sync)) -> Result<(), AppError> {
    let new = db::new_key();
    vault.save(KeySlot::Next, &new)?;
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
    vault.save(KeySlot::Current, &new)?;
    // Se falhar, a próxima abertura apaga a sobra (a atual já abre).
    let _ = vault.delete(KeySlot::Next);
    tracing::info!("chave do banco trocada");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Memory {
        keys: Mutex<HashMap<KeySlot, DbKey>>,
        fail_save: Mutex<Option<KeySlot>>,
    }

    impl Memory {
        fn get(&self, slot: KeySlot) -> Option<DbKey> {
            self.keys.lock().unwrap().get(&slot).cloned()
        }
    }

    impl KeyVault for Memory {
        fn load(&self, slot: KeySlot) -> Result<Option<DbKey>, AppError> {
            Ok(self.get(slot))
        }
        fn save(&self, slot: KeySlot, key: &DbKey) -> Result<(), AppError> {
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

    fn mark(db: &Db) {
        db.with(|c| c.execute_batch("INSERT INTO app_setting (key, value) VALUES ('marca', 'x')").map_err(db::sql_error("teste"))).unwrap();
    }

    fn has_mark(db: &Db) -> bool {
        db.with(|c| Ok(c.query_row("SELECT count(*) FROM app_setting WHERE key = 'marca'", [], |r| r.get::<_, i64>(0)).unwrap()))
            .unwrap()
            == 1
    }

    fn files(dir: &Path) -> usize {
        std::fs::read_dir(dir).unwrap().filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().contains("unreadable")).count()
    }

    #[test]
    fn first_open_creates_and_saves_a_key() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Memory::default();
        let db = open_database(dir.path(), &vault).unwrap();
        mark(&db);
        drop(db);
        assert!(vault.get(KeySlot::Current).is_some());
        assert!(has_mark(&open_database(dir.path(), &vault).unwrap()));
    }

    #[tokio::test]
    async fn rotate_keeps_data_and_leaves_only_the_new_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(db::FILE_NAME);
        let vault = Memory::default();
        let db = open_database(dir.path(), &vault).unwrap();
        mark(&db);
        let old = vault.get(KeySlot::Current).unwrap();
        rotate(&db, &path, &vault).await.unwrap();
        drop(db);
        let new = vault.get(KeySlot::Current).unwrap();
        assert_ne!(new, old);
        assert!(vault.get(KeySlot::Next).is_none());
        assert!(!db::opens_with(&path, &old), "a chave antiga não abre mais");
        assert!(has_mark(&open_database(dir.path(), &vault).unwrap()));
        assert_eq!(files(dir.path()), 0, "nada foi posto de lado");
    }

    #[test]
    fn interrupted_after_rekey_is_completed_on_open() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Memory::default();
        let db = open_database(dir.path(), &vault).unwrap();
        mark(&db);
        // O app fechou depois do rekey e antes de gravar a chave nova como atual.
        let new = db::new_key();
        vault.save(KeySlot::Next, &new).unwrap();
        db.with(|c| Db::rekey(c, &new)).unwrap();
        drop(db);
        let db = open_database(dir.path(), &vault).unwrap();
        assert!(has_mark(&db));
        assert_eq!(vault.get(KeySlot::Current), Some(new));
        assert!(vault.get(KeySlot::Next).is_none());
        assert_eq!(files(dir.path()), 0);
    }

    #[test]
    fn interrupted_before_rekey_is_undone_on_open() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Memory::default();
        let db = open_database(dir.path(), &vault).unwrap();
        mark(&db);
        drop(db);
        let old = vault.get(KeySlot::Current).unwrap();
        vault.save(KeySlot::Next, &db::new_key()).unwrap();
        let db = open_database(dir.path(), &vault).unwrap();
        assert!(has_mark(&db));
        assert_eq!(vault.get(KeySlot::Current), Some(old));
        assert!(vault.get(KeySlot::Next).is_none());
    }

    #[tokio::test]
    async fn failure_saving_the_new_key_never_loses_the_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(db::FILE_NAME);
        let vault = Memory::default();
        let db = open_database(dir.path(), &vault).unwrap();
        mark(&db);
        *vault.fail_save.lock().unwrap() = Some(KeySlot::Current);
        assert!(rotate(&db, &path, &vault).await.is_err());
        drop(db);
        *vault.fail_save.lock().unwrap() = None;
        let db = open_database(dir.path(), &vault).unwrap();
        assert!(has_mark(&db), "a abertura usa a chave nova guardada como próxima");
        assert_eq!(files(dir.path()), 0);
    }

    #[tokio::test]
    async fn failure_saving_the_next_key_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(db::FILE_NAME);
        let vault = Memory::default();
        let db = open_database(dir.path(), &vault).unwrap();
        mark(&db);
        let old = vault.get(KeySlot::Current).unwrap();
        *vault.fail_save.lock().unwrap() = Some(KeySlot::Next);
        assert!(rotate(&db, &path, &vault).await.is_err());
        drop(db);
        assert!(db::opens_with(&path, &old));
        *vault.fail_save.lock().unwrap() = None;
        assert!(has_mark(&open_database(dir.path(), &vault).unwrap()));
    }
}
