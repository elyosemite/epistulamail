//! Segredos no cofre do SO (Credential Manager, Keychain, Secret Service): refresh token de cada
//! conta e a chave do banco. Nada disso vai para o banco nem para o log.
//!
//! O Windows limita cada segredo a 2.560 bytes e o crate `keyring` grava em UTF-16 (até 1.280
//! caracteres), então o refresh token é dividido em partes: `account-<id>` guarda quantas são e
//! `secret-<id>-N` guarda cada parte. `accounts-index` lista as contas para recuperar o acesso
//! se o banco precisar ser recriado (CA1).
//!
//! Tudo que sai do cofre fica em `Zeroizing`, zerado ao sair de uso (US10, CA5). As cópias feitas
//! dentro do crate `keyring` e do Windows estão fora do alcance do app.

use keyring::Entry;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use crate::db::DbKey;
use crate::error::AppError;
use crate::provider::Provider;

const SERVICE: &str = "epistula";
const CHUNK_CHARS: usize = 1000;
const DB_KEY_ENTRY: &str = "db-key";
/// Chave nova durante uma troca (US10, CA4), até o banco abrir com ela.
const DB_KEY_NEXT_ENTRY: &str = "db-key-next";
const INDEX_ENTRY: &str = "accounts-index";
const LEGACY_ACCOUNT_ENTRY: &str = "account";

fn entry(name: &str) -> Result<Entry, AppError> {
    Entry::new(SERVICE, name).map_err(keyring_error)
}

fn keyring_error(e: keyring::Error) -> AppError {
    tracing::error!(error = %e, "falha no keyring do sistema");
    AppError::Keyring
}

/// Texto secreto que é zerado ao sair de uso.
pub type SecretString = Zeroizing<String>;

fn get(name: &str) -> Result<Option<SecretString>, AppError> {
    match entry(name)?.get_password() {
        Ok(value) => Ok(Some(Zeroizing::new(value))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(keyring_error(e)),
    }
}

fn set(name: &str, value: &str) -> Result<(), AppError> {
    entry(name)?.set_password(value).map_err(keyring_error)
}

fn remove(name: &str) -> Result<(), AppError> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(keyring_error(e)),
    }
}

pub fn split_secret(secret: &str) -> Vec<String> {
    let chars: Vec<char> = secret.chars().collect();
    chars.chunks(CHUNK_CHARS).map(|c| c.iter().collect()).collect()
}

fn meta_name(account_id: &str) -> String {
    format!("account-{account_id}")
}

fn part_name(account_id: &str, i: usize) -> String {
    format!("secret-{account_id}-{i}")
}

#[derive(Serialize, Deserialize)]
struct SecretMeta {
    parts: usize,
}

// ---------------------------------------------------------------- refresh token por conta

pub fn load_refresh_token(account_id: &str) -> Result<Option<SecretString>, AppError> {
    let Some(meta) = get(&meta_name(account_id))? else { return Ok(None) };
    let Ok(meta) = serde_json::from_str::<SecretMeta>(&meta) else { return Ok(None) };
    // Capacidade reservada antes, para o texto não ser realocado (e copiado) ao crescer.
    let mut token = Zeroizing::new(String::with_capacity(meta.parts * CHUNK_CHARS * 4));
    for i in 0..meta.parts {
        match get(&part_name(account_id, i))? {
            Some(part) => token.push_str(&part),
            None => return Ok(None),
        }
    }
    Ok(Some(token).filter(|t| !t.is_empty()))
}

/// Grava o refresh token e confere lendo de volta.
pub fn save_refresh_token(account_id: &str, token: &str) -> Result<(), AppError> {
    delete_refresh_token(account_id)?;
    let mut parts = split_secret(token);
    let written: Result<(), AppError> = parts.iter().enumerate().try_for_each(|(i, part)| set(&part_name(account_id, i), part));
    let count = parts.len();
    parts.zeroize();
    written?;
    let meta = serde_json::to_string(&SecretMeta { parts: count }).map_err(|_| AppError::Keyring)?;
    set(&meta_name(account_id), &meta)?;
    if load_refresh_token(account_id)?.as_deref().map(String::as_str) != Some(token) {
        tracing::error!("o refresh token gravado não confere na releitura");
        return Err(AppError::Keyring);
    }
    Ok(())
}

pub fn delete_refresh_token(account_id: &str) -> Result<(), AppError> {
    let parts = get(&meta_name(account_id))?
        .and_then(|m| serde_json::from_str::<SecretMeta>(&m).ok())
        .map(|m| m.parts)
        .unwrap_or(0);
    for i in 0..parts {
        remove(&part_name(account_id, i))?;
    }
    remove(&meta_name(account_id))
}

// ---------------------------------------------------------------- índice de contas

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexEntry {
    pub id: String,
    pub provider: String,
    pub email: String,
}

pub fn load_index() -> Result<Vec<IndexEntry>, AppError> {
    Ok(get(INDEX_ENTRY)?.and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default())
}

pub fn save_index(entries: &[IndexEntry]) -> Result<(), AppError> {
    let json = serde_json::to_string(entries).map_err(|_| AppError::Keyring)?;
    set(INDEX_ENTRY, &json)
}

// ---------------------------------------------------------------- chave do banco

/// Qual das chaves do banco: a atual ou a nova durante uma troca (US10, CA4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeySlot {
    Current,
    Next,
}

fn slot_name(slot: KeySlot) -> &'static str {
    match slot {
        KeySlot::Current => DB_KEY_ENTRY,
        KeySlot::Next => DB_KEY_NEXT_ENTRY,
    }
}

/// Como a chave do banco fica guardada no cofre, com versão (US10, CA2).
/// - v1: a chave crua em hexadecimal (64 caracteres), o formato de antes da US10.
/// - v2: a chave envolvida (AES-256-GCM) por uma chave derivada da senha do Epistula com Argon2id,
///   em JSON com os parâmetros, o sal, o nonce e o texto cifrado, para ser aberta só com a senha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredKey {
    Raw(DbKey),
    Wrapped(WrappedKey),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappedKey {
    pub v: u8,
    pub kdf: String,
    /// Memória do Argon2id em KiB, iterações e paralelismo.
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
    /// Sal, nonce e texto cifrado (chave + tag) em base64.
    pub salt: String,
    pub nonce: String,
    pub ct: String,
}

pub const WRAPPED_VERSION: u8 = 2;

impl StoredKey {
    pub fn parse(text: &str) -> Option<StoredKey> {
        if let Some(key) = parse_key(text) {
            return Some(StoredKey::Raw(key));
        }
        serde_json::from_str::<WrappedKey>(text).ok().filter(|w| w.v == WRAPPED_VERSION && w.kdf == "argon2id").map(StoredKey::Wrapped)
    }

    pub fn encode(&self) -> Result<SecretString, AppError> {
        match self {
            StoredKey::Raw(key) => Ok(key.to_hex()),
            StoredKey::Wrapped(w) => serde_json::to_string(w).map(Zeroizing::new).map_err(|_| AppError::Keyring),
        }
    }
}

pub fn load_db_key(slot: KeySlot) -> Result<Option<StoredKey>, AppError> {
    Ok(get(slot_name(slot))?.and_then(|text| StoredKey::parse(&text)))
}

/// Grava e confere lendo de volta.
pub fn save_db_key(slot: KeySlot, key: &StoredKey) -> Result<(), AppError> {
    set(slot_name(slot), &key.encode()?)?;
    if load_db_key(slot)?.as_ref() != Some(key) {
        tracing::error!(?slot, "a chave do banco gravada não confere na releitura");
        return Err(AppError::Keyring);
    }
    Ok(())
}

pub fn delete_db_key(slot: KeySlot) -> Result<(), AppError> {
    remove(slot_name(slot))
}

pub fn parse_key(hex: &str) -> Option<DbKey> {
    if hex.len() != 64 || !hex.is_ascii() {
        return None;
    }
    let mut key = DbKey::zeroed();
    for (i, byte) in key.bytes_mut().iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(key)
}

// ---------------------------------------------------------------- conta única da US1/US2

pub struct LegacyAccount {
    pub provider: Provider,
    pub email: String,
    pub refresh_token: String,
}

#[derive(Deserialize)]
struct LegacyMeta {
    provider: Provider,
    email: String,
    parts: usize,
}

fn legacy_part(i: usize) -> String {
    format!("secret-{i}")
}

pub fn load_legacy() -> Result<Option<LegacyAccount>, AppError> {
    let Some(meta) = get(LEGACY_ACCOUNT_ENTRY)? else { return Ok(None) };
    let Ok(meta) = serde_json::from_str::<LegacyMeta>(&meta) else { return Ok(None) };
    let mut token = String::new();
    for i in 0..meta.parts {
        match get(&legacy_part(i))? {
            Some(part) => token.push_str(&part),
            None => return Ok(None),
        }
    }
    if token.is_empty() {
        return Ok(None);
    }
    Ok(Some(LegacyAccount { provider: meta.provider, email: meta.email, refresh_token: token }))
}

pub fn delete_legacy() -> Result<(), AppError> {
    let parts = get(LEGACY_ACCOUNT_ENTRY)?
        .and_then(|m| serde_json::from_str::<LegacyMeta>(&m).ok())
        .map(|m| m.parts)
        .unwrap_or(0);
    for i in 0..parts {
        remove(&legacy_part(i))?;
    }
    remove(LEGACY_ACCOUNT_ENTRY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_secret_respects_windows_limit_and_roundtrips() {
        let secret = "x".repeat(2500);
        let parts = split_secret(&secret);
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|p| p.encode_utf16().count() * 2 <= 2560));
        assert_eq!(parts.concat(), secret);
    }

    #[test]
    fn split_secret_of_short_secret_is_single_part() {
        assert_eq!(split_secret("abc"), vec!["abc".to_string()]);
    }

    #[test]
    fn entry_names_are_scoped_by_account() {
        assert_eq!(meta_name("a1"), "account-a1");
        assert_eq!(part_name("a1", 2), "secret-a1-2");
        assert_ne!(part_name("a1", 0), legacy_part(0));
    }

    #[test]
    fn db_key_hex_roundtrips_and_rejects_garbage() {
        let key = crate::db::new_key();
        assert_eq!(parse_key(&key.to_hex()), Some(key));
        assert_eq!(parse_key("abc"), None);
        assert_eq!(parse_key(&"zz".repeat(32)), None);
        assert_eq!(parse_key(&"é".repeat(32)), None);
    }

    #[test]
    fn stored_key_reads_both_versions() {
        let key = crate::db::new_key();
        let raw = StoredKey::Raw(key.clone());
        assert_eq!(StoredKey::parse(&raw.encode().unwrap()), Some(raw), "v1: a chave de antes da US10");
        let wrapped = StoredKey::Wrapped(WrappedKey {
            v: WRAPPED_VERSION,
            kdf: "argon2id".into(),
            m_kib: 65536,
            t: 3,
            p: 1,
            salt: "c2Fs".into(),
            nonce: "bm9uY2U=".into(),
            ct: "Y3Q=".into(),
        });
        assert_eq!(StoredKey::parse(&wrapped.encode().unwrap()), Some(wrapped));
        assert_eq!(StoredKey::parse(r#"{"v":9,"kdf":"argon2id","m_kib":1,"t":1,"p":1,"salt":"","nonce":"","ct":""}"#), None, "versão desconhecida");
        assert_eq!(StoredKey::parse("lixo"), None);
        assert_eq!(slot_name(KeySlot::Current), "db-key", "o nome de antes da US10 não muda");
    }
}
