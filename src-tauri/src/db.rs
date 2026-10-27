//! Banco local cifrado com SQLCipher (US3, CA1). O arquivo inteiro é cifrado com AES-256; a chave
//! (32 bytes do gerador do SO) fica só no keyring. Migrações versionadas por `PRAGMA user_version`.
//! O esquema de cifra fica fixo no código e a chave pode ser trocada sem perder dados (US10, CA4).

use std::future::Future;
use std::panic::Location;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rand::rngs::OsRng;
use rand::RngCore;
use rusqlite::Connection;
use zeroize::Zeroizing;

use crate::error::AppError;

pub const FILE_NAME: &str = "epistula.db";

/// Migrações em ordem. A posição + 1 é a versão gravada em `user_version` depois de aplicada.
pub const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_inicial.sql"),
    include_str!("../migrations/0002_comandos.sql"),
    include_str!("../migrations/0003_sincronia_e_foto.sql"),
    include_str!("../migrations/0004_envio_e_mover.sql"),
    include_str!("../migrations/0005_conversas_e_notificacoes.sql"),
    include_str!("../migrations/0006_cifra.sql"),
    include_str!("../migrations/0007_regras.sql"),
    include_str!("../migrations/0008_indice_conversas.sql"),
    include_str!("../migrations/0009_openpgp.sql"),
    include_str!("../migrations/0010_smime.sql"),
    include_str!("../migrations/0011_arquivos.sql"),
    include_str!("../migrations/0012_busca.sql"),
];

/// Versão do esquema de cifra gravada em `app_setting` (`cipher_scheme`). 1 = SQLCipher 4 com chave
/// crua de 256 bits e as configurações de `CIPHER_PRAGMAS`. Uma troca futura parte desse número.
pub const CIPHER_SCHEME: u32 = 1;

/// Configurações de cifra explícitas, iguais ao padrão do SQLCipher 4 com que todo banco do
/// Epistula foi criado. Não dependem de uma mudança de padrão na biblioteca; um teste falha se
/// mudarem. Vêm logo depois da chave, antes da primeira leitura.
const CIPHER_PRAGMAS: &str = "PRAGMA cipher_compatibility = 4;
     PRAGMA cipher_page_size = 4096;
     PRAGMA kdf_iter = 256000;
     PRAGMA cipher_hmac_algorithm = HMAC_SHA512;
     PRAGMA cipher_kdf_algorithm = PBKDF2_HMAC_SHA512;";

/// Chave do banco. Zerada ao sair de uso e nunca exibida em `Debug` (US10, CA5).
#[derive(Clone, PartialEq, Eq)]
pub struct DbKey(Zeroizing<[u8; 32]>);

impl DbKey {
    pub fn zeroed() -> DbKey {
        DbKey(Zeroizing::new([0u8; 32]))
    }

    pub fn bytes_mut(&mut self) -> &mut [u8; 32] {
        &mut self.0
    }

    /// Hexadecimal minúsculo, num texto que também é zerado.
    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> Zeroizing<String> {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut hex = Zeroizing::new(String::with_capacity(64));
        for b in self.0.iter() {
            hex.push(DIGITS[(b >> 4) as usize] as char);
            hex.push(DIGITS[(b & 0xf) as usize] as char);
        }
        hex
    }
}

impl std::fmt::Debug for DbKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DbKey(…)")
    }
}

pub fn new_key() -> DbKey {
    let mut key = DbKey::zeroed();
    OsRng.fill_bytes(key.bytes_mut());
    key
}

#[derive(Debug, PartialEq, Eq)]
pub enum OpenError {
    /// A chave não abre o arquivo (ou o arquivo não é um banco SQLCipher).
    WrongKey,
    Other,
}

/// Conexão única, protegida por mutex. Toda chamada a partir de código assíncrono passa por
/// `call`, que roda a consulta fora das threads do runtime. Com o app bloqueado (US10, CA3) não há
/// conexão: o arquivo fica fechado e toda chamada devolve `Locked`.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Option<Connection>>>,
    /// De onde veio o último acesso que pegou a conexão, para o log dizer quem fez esperar.
    holder: Arc<Mutex<Option<&'static Location<'static>>>>,
}

/// Acima disso, a espera ou a consulta vai para o log.
const SLOW: Duration = Duration::from_millis(100);

/// `offline.rs:111`: só o arquivo e a linha do código, nunca dados.
fn place(at: &Location<'_>) -> String {
    let file = at.file().rsplit(['/', '\\']).next().unwrap_or_default();
    format!("{file}:{}", at.line())
}

pub fn sql_error(context: &'static str) -> impl Fn(rusqlite::Error) -> AppError {
    move |e| {
        // O texto de erro do SQLite pode citar valores; só o código vai para o log.
        let code = e.sqlite_error_code().map(|c| format!("{c:?}")).unwrap_or_else(|| "outro".into());
        tracing::error!(code, context, "falha no banco local");
        AppError::Storage
    }
}

fn key_pragma(name: &str, key: &DbKey) -> Zeroizing<String> {
    let mut sql = Zeroizing::new(String::with_capacity(96));
    sql.push_str("PRAGMA ");
    sql.push_str(name);
    sql.push_str(" = \"x'");
    sql.push_str(&key.to_hex());
    sql.push_str("'\";");
    sql
}

/// Abre o arquivo com a chave e as configurações de cifra. `WrongKey` quando a chave não abre.
fn connect(path: &Path, key: &DbKey) -> Result<Connection, OpenError> {
    let conn = Connection::open(path).map_err(|_| OpenError::Other)?;
    // `cipher_memory_security = ON` (zerar a memória ao liberar) estoura a pilha dentro do SQLCipher no
    // Windows (testado na US10); fica desligado de forma explícita. As chaves do app são zeradas por `zeroize`.
    conn.execute_batch("PRAGMA cipher_memory_security = OFF;").map_err(|_| OpenError::Other)?;
    conn.execute_batch(&key_pragma("key", key)).map_err(|_| OpenError::Other)?;
    conn.execute_batch(CIPHER_PRAGMAS).map_err(|_| OpenError::Other)?;
    // Com a chave errada o SQLCipher só falha na primeira leitura.
    if conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0)).is_err() {
        return Err(OpenError::WrongKey);
    }
    Ok(conn)
}

/// Confere, numa conexão separada, se a chave abre o arquivo.
pub fn opens_with(path: &Path, key: &DbKey) -> bool {
    connect(path, key).is_ok()
}

impl Db {
    /// Abre (ou cria) o banco com a chave e aplica as migrações pendentes.
    pub fn open(path: &Path, key: &DbKey) -> Result<Db, OpenError> {
        let db = Self::open_with(path, key, MIGRATIONS)?;
        let scheme = db.with_conn(|c| cipher_scheme(c)).map_err(|_| OpenError::Other)?;
        if scheme.is_some_and(|s| s > CIPHER_SCHEME) {
            tracing::error!(scheme, known = CIPHER_SCHEME, "banco cifrado por uma versão mais nova do Epistula");
            return Err(OpenError::Other);
        }
        Ok(db)
    }

    pub fn open_with(path: &Path, key: &DbKey, migrations: &[&str]) -> Result<Db, OpenError> {
        let conn = connect(path, key)?;
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
        Ok(Db { conn: Arc::new(Mutex::new(Some(conn))), holder: Arc::default() })
    }

    /// Sem conexão: o app abriu bloqueado e espera a senha.
    pub fn closed() -> Db {
        Db { conn: Arc::new(Mutex::new(None)), holder: Arc::default() }
    }

    /// Passa para este `Db` (o que o app inteiro usa) a conexão de um banco recém-aberto.
    pub fn attach(&self, opened: Db) {
        let conn = opened.conn.lock().ok().and_then(|mut c| c.take());
        if let Ok(mut mine) = self.conn.lock() {
            *mine = conn;
        }
    }

    /// Fecha o arquivo; a próxima chamada devolve `Locked`.
    pub fn close(&self) {
        if let Ok(mut mine) = self.conn.lock() {
            mine.take();
        }
    }

    pub fn is_open(&self) -> bool {
        self.conn.lock().map(|c| c.is_some()).unwrap_or(false)
    }

    fn with_conn<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T, AppError>) -> Result<T, AppError> {
        let mut guard = self.conn.lock().map_err(|_| AppError::Storage)?;
        f(guard.as_mut().ok_or(AppError::Locked)?)
    }

    /// Cifra o banco inteiro com a chave nova (`PRAGMA rekey`), numa transação só: se falhar no
    /// meio, o arquivo continua abrindo com a chave antiga. Guardar a chave é com `keys::rotate`.
    pub fn rekey(conn: &mut Connection, key: &DbKey) -> Result<(), AppError> {
        conn.execute_batch(&key_pragma("rekey", key)).map_err(sql_error("rekey"))?;
        // Confere na própria conexão antes de devolver.
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0)).map_err(sql_error("rekey"))?;
        Ok(())
    }

    /// Roda `f` com a conexão, fora das threads do runtime. Espera ou consulta lenta vai para o log
    /// com o lugar no código de quem chamou e de quem estava antes com a conexão (só arquivo e linha).
    #[track_caller]
    pub fn call<T, F>(&self, f: F) -> impl Future<Output = Result<T, AppError>> + Send + 'static
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T, AppError> + Send + 'static,
    {
        let at = Location::caller();
        let (conn, holder) = (self.conn.clone(), self.holder.clone());
        async move {
            tokio::task::spawn_blocking(move || {
                let asked = Instant::now();
                let mut guard = conn.lock().map_err(|_| AppError::Storage)?;
                let waited = asked.elapsed();
                let before = holder.lock().ok().and_then(|mut h| h.replace(at));
                let result = f(guard.as_mut().ok_or(AppError::Locked)?);
                let ran = asked.elapsed() - waited;
                drop(guard);
                if waited >= SLOW {
                    let after = before.map(place).unwrap_or_default();
                    tracing::info!(wait_ms = waited.as_millis() as u64, run_ms = ran.as_millis() as u64, at = %place(at), after = %after, "acesso ao banco esperou outro");
                }
                if ran >= SLOW {
                    tracing::info!(run_ms = ran.as_millis() as u64, at = %place(at), "acesso lento ao banco");
                }
                result
            })
            .await
            .map_err(|_| AppError::Storage)?
        }
    }

    /// Acesso síncrono, para testes.
    #[cfg(test)]
    pub fn with<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T, AppError>) -> Result<T, AppError> {
        self.with_conn(f)
    }
}

/// Esquema de cifra registrado no banco (`None` antes da migração 0006).
pub fn cipher_scheme(conn: &Connection) -> Result<Option<u32>, AppError> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM app_setting WHERE key = 'cipher_scheme'", [], |r| r.get(0))
        .map(Some)
        .or_else(|e| if matches!(e, rusqlite::Error::QueryReturnedNoRows) { Ok(None) } else { Err(e) })
        .map_err(sql_error("cipher_scheme"))?;
    Ok(value.and_then(|v| v.parse().ok()))
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

    fn pragma(db: &Db, name: &str) -> String {
        db.with(|c| Ok(c.query_row(&format!("PRAGMA {name}"), [], |r| r.get::<_, String>(0)).unwrap())).unwrap()
    }

    #[test]
    fn cipher_settings_are_pinned() {
        let (_dir, path) = temp_path();
        let db = Db::open(&path, &new_key()).unwrap();
        // Mudar qualquer um destes valores deixa os bancos atuais ilegíveis: exige um esquema novo.
        assert_eq!(pragma(&db, "cipher_page_size"), "4096");
        assert_eq!(pragma(&db, "kdf_iter"), "256000");
        assert_eq!(pragma(&db, "cipher_hmac_algorithm"), "HMAC_SHA512");
        assert_eq!(pragma(&db, "cipher_kdf_algorithm"), "PBKDF2_HMAC_SHA512");
        assert_eq!(pragma(&db, "cipher_memory_security"), "0");
        assert_eq!(db.with(|c| cipher_scheme(c)).unwrap(), Some(CIPHER_SCHEME));
    }

    #[test]
    fn database_created_with_library_defaults_still_opens() {
        let (_dir, path) = temp_path();
        let key = new_key();
        {
            // Como os bancos eram criados antes da US10: só a chave, com o padrão do SQLCipher.
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(&key_pragma("key", &key)).unwrap();
            conn.execute_batch("CREATE TABLE antiga (x INTEGER); INSERT INTO antiga VALUES (7);").unwrap();
        }
        let db = Db::open_with(&path, &key, &["SELECT 1;"]).unwrap();
        let x: i64 = db.with(|c| Ok(c.query_row("SELECT x FROM antiga", [], |r| r.get(0)).unwrap())).unwrap();
        assert_eq!(x, 7);
    }

    #[test]
    fn rekey_keeps_data_and_retires_the_old_key() {
        let (_dir, path) = temp_path();
        let (old, new) = (new_key(), new_key());
        let db = Db::open(&path, &old).unwrap();
        db.with(|c| c.execute_batch("INSERT INTO app_setting (key, value) VALUES ('marca', 'x')").map_err(sql_error("teste"))).unwrap();
        db.with(|c| Db::rekey(c, &new)).unwrap();
        drop(db);
        assert!(!opens_with(&path, &old), "a chave antiga não pode abrir depois da troca");
        let db = Db::open(&path, &new).unwrap();
        assert_eq!(db.with(|c| Ok(c.query_row("SELECT value FROM app_setting WHERE key = 'marca'", [], |r| r.get::<_, String>(0)).unwrap())).unwrap(), "x");
    }

    #[tokio::test]
    async fn closed_database_answers_locked_until_attached() {
        let (_dir, path) = temp_path();
        let key = new_key();
        let shared = Db::closed();
        assert!(matches!(shared.call(|_| Ok(())).await, Err(AppError::Locked)));
        shared.attach(Db::open(&path, &key).unwrap());
        assert!(shared.is_open());
        assert_eq!(shared.call(|c| Ok(c.query_row("SELECT 1", [], |r| r.get::<_, i64>(0)).unwrap())).await.unwrap(), 1);
        shared.close();
        assert!(!shared.is_open());
        assert!(matches!(shared.call(|_| Ok(())).await, Err(AppError::Locked)));
    }

    #[test]
    fn slow_access_log_names_only_file_and_line() {
        let at = Location::caller();
        assert_eq!(place(at), format!("db.rs:{}", at.line()));
    }

    #[test]
    fn key_never_shows_in_debug() {
        let key = new_key();
        let shown = format!("{key:?}");
        assert_eq!(shown, "DbKey(…)");
        assert!(!shown.contains(&key.to_hex()[..8]));
        assert_eq!(key.to_hex().len(), 64);
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
