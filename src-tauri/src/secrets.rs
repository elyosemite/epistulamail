//! Segredos no cofre do SO (Credential Manager, Keychain, Secret Service): refresh token de cada
//! conta e a chave do banco. Nada disso vai para o banco nem para o log.
//!
//! O Windows limita cada segredo a 2.560 bytes e o crate `keyring` grava em UTF-16 (até 1.280
//! caracteres), então o refresh token é dividido em partes: `account-<id>` guarda quantas são e
//! `secret-<id>-N` guarda cada parte. `accounts-index` lista as contas para recuperar o acesso
//! se o banco precisar ser recriado (CA1).

use keyring::Entry;
use serde::{Deserialize, Serialize};

use crate::db::DbKey;
use crate::error::AppError;
use crate::provider::Provider;

const SERVICE: &str = "epistula";
const CHUNK_CHARS: usize = 1000;
const DB_KEY_ENTRY: &str = "db-key";
const INDEX_ENTRY: &str = "accounts-index";
const LEGACY_ACCOUNT_ENTRY: &str = "account";

fn entry(name: &str) -> Result<Entry, AppError> {
    Entry::new(SERVICE, name).map_err(keyring_error)
}

fn keyring_error(e: keyring::Error) -> AppError {
    tracing::error!(error = %e, "falha no keyring do sistema");
    AppError::Keyring
}

fn get(name: &str) -> Result<Option<String>, AppError> {
    match entry(name)?.get_password() {
        Ok(value) => Ok(Some(value)),
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

pub fn load_refresh_token(account_id: &str) -> Result<Option<String>, AppError> {
    let Some(meta) = get(&meta_name(account_id))? else { return Ok(None) };
    let Ok(meta) = serde_json::from_str::<SecretMeta>(&meta) else { return Ok(None) };
    let mut token = String::new();
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
    let parts = split_secret(token);
    for (i, part) in parts.iter().enumerate() {
        set(&part_name(account_id, i), part)?;
    }
    let meta = serde_json::to_string(&SecretMeta { parts: parts.len() }).map_err(|_| AppError::Keyring)?;
    set(&meta_name(account_id), &meta)?;
    if load_refresh_token(account_id)?.as_deref() != Some(token) {
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

pub fn load_db_key() -> Result<Option<DbKey>, AppError> {
    Ok(get(DB_KEY_ENTRY)?.and_then(|hex| parse_key(&hex)))
}

pub fn save_db_key(key: &DbKey) -> Result<(), AppError> {
    let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
    set(DB_KEY_ENTRY, &hex)?;
    if load_db_key()?.as_ref() != Some(key) {
        tracing::error!("a chave do banco gravada não confere na releitura");
        return Err(AppError::Keyring);
    }
    Ok(())
}

pub fn parse_key(hex: &str) -> Option<DbKey> {
    if hex.len() != 64 || !hex.is_ascii() {
        return None;
    }
    let mut key = [0u8; 32];
    for (i, byte) in key.iter_mut().enumerate() {
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
        let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(parse_key(&hex), Some(key));
        assert_eq!(parse_key("abc"), None);
        assert_eq!(parse_key(&"zz".repeat(32)), None);
        assert_eq!(parse_key(&"é".repeat(32)), None);
    }
}
