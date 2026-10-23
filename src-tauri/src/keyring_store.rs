//! Persistência da conta no cofre do SO (Credential Manager, Keychain, Secret Service).
//!
//! O Windows limita cada segredo a 2.560 bytes e o crate `keyring` grava em UTF-16
//! (até 1.280 caracteres). Refresh tokens da Microsoft podem passar disso, então o
//! refresh token é dividido em partes: `account` guarda os metadados e `secret-N` as partes.

use keyring::Entry;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::provider::Provider;

const SERVICE: &str = "epistula";
const ACCOUNT_ENTRY: &str = "account";
const CHUNK_CHARS: usize = 1000;

pub struct StoredAccount {
    pub provider: Provider,
    pub email: String,
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize)]
struct AccountMeta {
    provider: Provider,
    email: String,
    parts: usize,
}

fn entry(name: &str) -> Result<Entry, AppError> {
    Entry::new(SERVICE, name).map_err(keyring_error)
}

fn part_name(i: usize) -> String {
    format!("secret-{i}")
}

fn keyring_error(e: keyring::Error) -> AppError {
    tracing::error!(error = %e, "falha no keyring do sistema");
    AppError::Keyring
}

pub fn split_secret(secret: &str) -> Vec<String> {
    let chars: Vec<char> = secret.chars().collect();
    chars.chunks(CHUNK_CHARS).map(|c| c.iter().collect()).collect()
}

pub fn load() -> Result<Option<StoredAccount>, AppError> {
    let meta = match entry(ACCOUNT_ENTRY)?.get_password() {
        Ok(json) => json,
        Err(keyring::Error::NoEntry) => return Ok(None),
        Err(e) => return Err(keyring_error(e)),
    };
    let Ok(meta) = serde_json::from_str::<AccountMeta>(&meta) else {
        tracing::warn!("metadados da conta em formato desconhecido; ignorando");
        return Ok(None);
    };
    let mut refresh_token = String::new();
    for i in 0..meta.parts {
        match entry(&part_name(i))?.get_password() {
            Ok(part) => refresh_token.push_str(&part),
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(e) => return Err(keyring_error(e)),
        }
    }
    if refresh_token.is_empty() {
        return Ok(None);
    }
    Ok(Some(StoredAccount { provider: meta.provider, email: meta.email, refresh_token }))
}

pub fn save(account: &StoredAccount) -> Result<(), AppError> {
    delete()?;
    let parts = split_secret(&account.refresh_token);
    for (i, part) in parts.iter().enumerate() {
        entry(&part_name(i))?.set_password(part).map_err(keyring_error)?;
    }
    let meta = AccountMeta {
        provider: account.provider,
        email: account.email.clone(),
        parts: parts.len(),
    };
    let json = serde_json::to_string(&meta).map_err(|_| AppError::Keyring)?;
    entry(ACCOUNT_ENTRY)?.set_password(&json).map_err(keyring_error)
}

pub fn delete() -> Result<(), AppError> {
    let parts = match entry(ACCOUNT_ENTRY)?.get_password() {
        Ok(json) => serde_json::from_str::<AccountMeta>(&json).map(|m| m.parts).unwrap_or(0),
        Err(keyring::Error::NoEntry) => 0,
        Err(e) => return Err(keyring_error(e)),
    };
    for i in 0..parts {
        remove(&part_name(i))?;
    }
    remove(ACCOUNT_ENTRY)
}

fn remove(name: &str) -> Result<(), AppError> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(keyring_error(e)),
    }
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
}
