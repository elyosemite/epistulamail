//! Banco local cifrado com SQLCipher (US3, CA1). O arquivo inteiro é cifrado com AES-256; a chave
//! (32 bytes do gerador do SO) fica só no keyring. Migrações versionadas por `PRAGMA user_version`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rand::rngs::OsRng;
use rand::RngCore;
use rusqlite::Connection;

use crate::error::AppError;

pub const FILE_NAME: &str = "epistula.db";

/// Migrações em ordem. A posição + 1 é a versão gravada em `user_version` depois de aplicada.
pub const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_inicial.sql"),
    include_str!("../migrations/0002_comandos.sql"),
    include_str!("../migrations/0003_sincronia_e_foto.sql"),
];

pub type DbKey = [u8; 32];

pub fn new_key() -> DbKey {
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

#[derive(Debug, PartialEq, Eq)]
pub enum OpenError {
    /// A chave não abre o arquivo (ou o arquivo não é um banco SQLCipher).
    WrongKey,
    Other,
}

/// Conexão única, protegida por mutex. Toda chamada a partir de código assíncrono passa por
/// `call`, que roda a consulta fora das threads do runtime.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

pub fn sql_error(context: &'static str) -> impl Fn(rusqlite::Error) -> AppError {
    move |e| {
        // O texto de erro do SQLite pode citar valores; só o código vai para o log.
        let code = e.sqlite_error_code().map(|c| format!("{c:?}")).unwrap_or_else(|| "outro".into());
        tracing::error!(code, context, "falha no banco local");
        AppError::Storage
    }
}

fn key_pragma(key: &DbKey) -> String {
    let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
    format!("PRAGMA key = \"x'{hex}'\";")
}

impl Db {
    /// Abre (ou cria) o banco com a chave e aplica as migrações pendentes.
    pub fn open(path: &Path, key: &DbKey) -> Result<Db, OpenError> {
        Self::open_with(path, key, MIGRATIONS)
    }

    pub fn open_with(path: &Path, key: &DbKey, migrations: &[&str]) -> Result<Db, OpenError> {
        let conn = Connection::open(path).map_err(|_| OpenError::Other)?;
        conn.execute_batch(&key_pragma(key)).map_err(|_| OpenError::Other)?;
        // Com a chave errada o SQLCipher só falha na primeira leitura.
        if conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0)).is_err() {
            return Err(OpenError::WrongKey);
        }
        conn.execute_batch(
            "PRAGMA auto_vacuum = INCREMENTAL;
             PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA secure_delete = ON;
             PRAGMA temp_store = MEMORY;",
        )
        .map_err(|_| OpenError::Other)?;
        let mut conn = conn;
        migrate(&mut conn, migrations).map_err(|_| OpenError::Other)?;
        Ok(Db { conn: Arc::new(Mutex::new(conn)) })
    }

    pub async fn call<T, F>(&self, f: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T, AppError> + Send + 'static,
    {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let asked = std::time::Instant::now();
            let mut guard = conn.lock().map_err(|_| AppError::Storage)?;
            let waited = asked.elapsed();
            let result = f(&mut guard);
            let ran = asked.elapsed() - waited;
            // Diagnóstico da abertura: espera pela conexão única e consultas lentas (só tempos).
            if waited.as_millis() >= 100 || ran.as_millis() >= 100 {
                tracing::info!(wait_ms = waited.as_millis() as u64, run_ms = ran.as_millis() as u64, "acesso lento ao banco");
            }
            result
        })
        .await
        .map_err(|_| AppError::Storage)?
    }

    /// Acesso síncrono, para testes.
    #[cfg(test)]
    pub fn with<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T, AppError>) -> Result<T, AppError> {
        let mut guard = self.conn.lock().map_err(|_| AppError::Storage)?;
        f(&mut guard)
    }
}

/// Aplica, cada uma na sua transação, as migrações acima de `user_version`. Uma migração que
/// falha é desfeita inteira e a versão não muda.
pub fn migrate(conn: &mut Connection, migrations: &[&str]) -> Result<(), AppError> {
    let current: usize = conn
        .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
        .map_err(sql_error("user_version"))?
        .try_into()
        .unwrap_or(0);
    if current > migrations.len() {
        tracing::error!(current, known = migrations.len(), "banco criado por uma versão mais nova do Epistula");
        return Err(AppError::Storage);
    }
    for (index, sql) in migrations.iter().enumerate().skip(current) {
        let tx = conn.transaction().map_err(sql_error("migração"))?;
        tx.execute_batch(sql).map_err(sql_error("migração"))?;
        tx.pragma_update(None, "user_version", (index + 1) as i64).map_err(sql_error("migração"))?;
        tx.commit().map_err(sql_error("migração"))?;
        tracing::info!(version = index + 1, "migração aplicada");
    }
    Ok(())
}

/// Renomeia o banco ilegível (e o WAL) para `epistula.db.unreadable-<segundos>`, sem apagar nada.
pub fn set_aside(path: &Path) -> Option<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let target = PathBuf::from(format!("{}.unreadable-{stamp}", path.display()));
    if std::fs::rename(path, &target).is_err() {
        tracing::error!("não foi possível pôr de lado o banco ilegível");
        return None;
    }
    for ext in ["-wal", "-shm"] {
        let side = PathBuf::from(format!("{}{ext}", path.display()));
        if side.exists() {
            let _ = std::fs::rename(&side, format!("{}{ext}", target.display()));
        }
    }
    tracing::warn!("banco ilegível posto de lado; um banco novo será criado");
    Some(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        (dir, path)
    }

    fn version(db: &Db) -> i64 {
        db.with(|c| Ok(c.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap())).unwrap()
    }

    #[test]
    fn migrates_from_scratch_and_reopens() {
        let (_dir, path) = temp_path();
        let key = new_key();
        let db = Db::open(&path, &key).unwrap();
        assert_eq!(version(&db), MIGRATIONS.len() as i64);
        let fk: i64 = db.with(|c| Ok(c.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap())).unwrap();
        assert_eq!(fk, 1);
        drop(db);
        let again = Db::open(&path, &key).unwrap();
        assert_eq!(version(&again), MIGRATIONS.len() as i64);
    }

    #[test]
    fn wrong_key_is_detected() {
        let (_dir, path) = temp_path();
        drop(Db::open(&path, &new_key()).unwrap());
        assert_eq!(Db::open(&path, &new_key()).err(), Some(OpenError::WrongKey));
    }

    #[test]
    fn failed_migration_keeps_previous_version() {
        let (_dir, path) = temp_path();
        let key = new_key();
        let ok = "CREATE TABLE a (x INTEGER);";
        drop(Db::open_with(&path, &key, &[ok]).unwrap());
        let broken = "CREATE TABLE b (y INTEGER); INSERT INTO tabela_que_nao_existe VALUES (1);";
        assert!(Db::open_with(&path, &key, &[ok, broken]).is_err());
        let db = Db::open_with(&path, &key, &[ok]).unwrap();
        assert_eq!(version(&db), 1);
        let b_exists: i64 = db
            .with(|c| Ok(c.query_row("SELECT count(*) FROM sqlite_master WHERE name = 'b'", [], |r| r.get(0)).unwrap()))
            .unwrap();
        assert_eq!(b_exists, 0, "a migração quebrada não pode deixar a tabela b");
    }

    #[test]
    fn newer_database_is_refused() {
        let (_dir, path) = temp_path();
        let key = new_key();
        drop(Db::open_with(&path, &key, &["SELECT 1;", "SELECT 1;"]).unwrap());
        assert!(Db::open_with(&path, &key, &["SELECT 1;"]).is_err());
    }

    #[test]
    fn set_aside_keeps_the_old_file() {
        let (dir, path) = temp_path();
        drop(Db::open(&path, &new_key()).unwrap());
        let moved = set_aside(&path).unwrap();
        assert!(!path.exists());
        assert!(moved.exists());
        assert!(moved.starts_with(dir.path()));
    }
}
