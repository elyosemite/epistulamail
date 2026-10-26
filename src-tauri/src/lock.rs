//! Senha do Epistula e bloqueio (US10, CA2 e CA3). A senha nunca é gravada: o Argon2id (RFC 9106)
//! deriva dela uma chave de 256 bits (`Kek`) que envolve, com AES-256-GCM, a chave do banco e os
//! refresh tokens no cofre. Sem a senha, nem a chave do cofre nem o backup do DPAPI em domínio
//! abrem o banco ou dão acesso ao servidor. Logs: só eventos e quantidades, nunca senha ou chave.

use std::time::{Duration, Instant};

use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::Zeroizing;

use crate::db::DbKey;
use crate::error::AppError;
use crate::secrets::{WrappedKey, WRAPPED_VERSION};

/// Perfil da RFC 9106 para desktop: 64 MiB de memória e 3 passadas (cerca de meio segundo).
pub const DEFAULT_COST: Cost = Cost { m_kib: 64 * 1024, t: 3, p: 1 };
/// Senha mínima, em caracteres.
pub const MIN_PASSWORD_CHARS: usize = 8;
/// Maior espera entre tentativas erradas.
const MAX_WAIT: Duration = Duration::from_secs(300);
/// Dados associados: amarram o texto cifrado ao uso, para um não servir no lugar do outro.
const AAD_DB_KEY: &[u8] = b"epistula/db-key/v2";
const AAD_TOKEN: &str = "epistula/refresh-token/v1/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

impl Cost {
    /// Custos aceitos ao ler o cofre: um valor adulterado não pode travar o app nem enfraquecer a
    /// derivação além do mínimo da RFC 9106.
    fn is_sane(&self) -> bool {
        (19 * 1024..=1024 * 1024).contains(&self.m_kib) && (1..=10).contains(&self.t) && (1..=4).contains(&self.p)
    }
}

/// Chave derivada da senha, com o sal e o custo usados. Zerada ao sair de uso.
#[derive(Clone)]
pub struct Kek {
    key: Zeroizing<[u8; 32]>,
    salt: Vec<u8>,
    cost: Cost,
}

impl std::fmt::Debug for Kek {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Kek(…)")
    }
}

fn derive_with(password: &str, salt: &[u8], cost: Cost) -> Result<Kek, AppError> {
    let params = Params::new(cost.m_kib, cost.t, cost.p, Some(32)).map_err(|_| AppError::InvalidInput)?;
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params).hash_password_into(password.as_bytes(), salt, &mut *key).map_err(|_| {
        tracing::error!("falha ao derivar a chave da senha");
        AppError::Storage
    })?;
    Ok(Kek { key, salt: salt.to_vec(), cost })
}

/// Chave nova para uma senha nova, com sal aleatório.
pub fn new_kek(password: &str) -> Result<Kek, AppError> {
    new_kek_with(password, DEFAULT_COST)
}

pub fn new_kek_with(password: &str, cost: Cost) -> Result<Kek, AppError> {
    if password.chars().count() < MIN_PASSWORD_CHARS {
        return Err(AppError::WeakPassword);
    }
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    derive_with(password, &salt, cost)
}

/// Deriva de novo a chave com o sal e o custo guardados junto da chave envolvida.
pub fn derive(password: &str, wrapped: &WrappedKey) -> Result<Kek, AppError> {
    let cost = Cost { m_kib: wrapped.m_kib, t: wrapped.t, p: wrapped.p };
    if !cost.is_sane() || wrapped.v != WRAPPED_VERSION {
        tracing::error!("parâmetros da chave envolvida fora do aceito");
        return Err(AppError::Keyring);
    }
    let salt = STANDARD.decode(&wrapped.salt).map_err(|_| AppError::Keyring)?;
    derive_with(password, &salt, cost)
}

fn cipher(kek: &Kek) -> Aes256Gcm {
    Aes256Gcm::new_from_slice(&kek.key[..]).expect("chave de 32 bytes")
}

fn seal(kek: &Kek, aad: &[u8], plain: &[u8]) -> Result<(Vec<u8>, Vec<u8>), AppError> {
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let ct = cipher(kek).encrypt(Nonce::from_slice(&nonce), Payload { msg: plain, aad }).map_err(|_| AppError::Storage)?;
    Ok((nonce.to_vec(), ct))
}

fn open(kek: &Kek, aad: &[u8], nonce: &[u8], ct: &[u8]) -> Result<Zeroizing<Vec<u8>>, AppError> {
    if nonce.len() != 12 {
        return Err(AppError::WrongPassword);
    }
    cipher(kek).decrypt(Nonce::from_slice(nonce), Payload { msg: ct, aad }).map(Zeroizing::new).map_err(|_| AppError::WrongPassword)
}

/// Envolve a chave do banco com a chave da senha.
pub fn wrap(kek: &Kek, key: &DbKey) -> Result<WrappedKey, AppError> {
    let (nonce, ct) = seal(kek, AAD_DB_KEY, key.bytes())?;
    Ok(WrappedKey {
        v: WRAPPED_VERSION,
        kdf: "argon2id".into(),
        m_kib: kek.cost.m_kib,
        t: kek.cost.t,
        p: kek.cost.p,
        salt: STANDARD.encode(&kek.salt),
        nonce: STANDARD.encode(nonce),
        ct: STANDARD.encode(ct),
    })
}

/// Abre a chave do banco. A tag do GCM confere a senha: senha errada é `WrongPassword`.
pub fn unwrap(kek: &Kek, wrapped: &WrappedKey) -> Result<DbKey, AppError> {
    if STANDARD.decode(&wrapped.salt).ok().as_deref() != Some(kek.salt.as_slice()) {
        return Err(AppError::WrongPassword);
    }
    let nonce = STANDARD.decode(&wrapped.nonce).map_err(|_| AppError::Keyring)?;
    let ct = STANDARD.decode(&wrapped.ct).map_err(|_| AppError::Keyring)?;
    let plain = open(kek, AAD_DB_KEY, &nonce, &ct)?;
    if plain.len() != 32 {
        return Err(AppError::Keyring);
    }
    let mut key = DbKey::zeroed();
    key.bytes_mut().copy_from_slice(&plain);
    Ok(key)
}

/// Cifra um refresh token para o cofre, amarrado à conta.
pub fn seal_token(kek: &Kek, account_id: &str, token: &str) -> Result<String, AppError> {
    let aad = format!("{AAD_TOKEN}{account_id}");
    let (nonce, ct) = seal(kek, aad.as_bytes(), token.as_bytes())?;
    let mut both = nonce;
    both.extend_from_slice(&ct);
    Ok(STANDARD.encode(both))
}

pub fn open_token(kek: &Kek, account_id: &str, sealed: &str) -> Result<Zeroizing<String>, AppError> {
    let both = STANDARD.decode(sealed).map_err(|_| AppError::WrongPassword)?;
    if both.len() < 12 {
        return Err(AppError::WrongPassword);
    }
    let aad = format!("{AAD_TOKEN}{account_id}");
    let plain = open(kek, aad.as_bytes(), &both[..12], &both[12..])?;
    String::from_utf8(plain.to_vec()).map(Zeroizing::new).map_err(|_| AppError::WrongPassword)
}

/// Espera antes da próxima tentativa depois de `failures` erros seguidos: 2, 4, 8… até 5 min.
pub fn wait_after(failures: u32) -> Duration {
    if failures == 0 {
        return Duration::ZERO;
    }
    Duration::from_secs(1u64 << failures.min(9)).min(MAX_WAIT)
}

/// Tentativas erradas desde o último desbloqueio (só em memória).
#[derive(Default)]
pub struct Attempts {
    failures: u32,
    retry_at: Option<Instant>,
}

impl Attempts {
    /// Segundos até poder tentar de novo (0 se já pode).
    pub fn wait_seconds(&self, now: Instant) -> u64 {
        self.retry_at.map_or(0, |at| at.saturating_duration_since(now).as_secs_f64().ceil() as u64)
    }

    pub fn failed(&mut self, now: Instant) {
        self.failures = self.failures.saturating_add(1);
        self.retry_at = Some(now + wait_after(self.failures));
    }

    pub fn succeeded(&mut self) {
        *self = Attempts::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::new_key;

    /// Custo mínimo aceito, para os testes não levarem segundos.
    const TEST_COST: Cost = Cost { m_kib: 19 * 1024, t: 1, p: 1 };

    #[test]
    fn wraps_and_unwraps_the_database_key() {
        let key = new_key();
        let kek = new_kek_with("frase longa e secreta", TEST_COST).unwrap();
        let wrapped = wrap(&kek, &key).unwrap();
        assert_eq!(wrapped.v, WRAPPED_VERSION);
        assert!(!wrapped.ct.contains(&key.to_hex()[..16]));
        let again = derive("frase longa e secreta", &wrapped).unwrap();
        assert_eq!(unwrap(&again, &wrapped).unwrap(), key);
    }

    #[test]
    fn wrong_password_and_tampering_are_refused() {
        let key = new_key();
        let kek = new_kek_with("frase longa e secreta", TEST_COST).unwrap();
        let wrapped = wrap(&kek, &key).unwrap();
        let wrong = derive("outra frase qualquer", &wrapped).unwrap();
        assert!(matches!(unwrap(&wrong, &wrapped), Err(AppError::WrongPassword)));
        let mut tampered = wrapped.clone();
        let mut ct = STANDARD.decode(&tampered.ct).unwrap();
        ct[0] ^= 1;
        tampered.ct = STANDARD.encode(ct);
        assert!(matches!(unwrap(&kek, &tampered), Err(AppError::WrongPassword)));
    }

    #[test]
    fn cost_outside_the_accepted_range_is_refused() {
        let kek = new_kek_with("frase longa e secreta", TEST_COST).unwrap();
        let mut wrapped = wrap(&kek, &new_key()).unwrap();
        wrapped.m_kib = 8;
        assert!(derive("frase longa e secreta", &wrapped).is_err(), "custo baixo demais");
        wrapped.m_kib = 64 * 1024 * 1024;
        assert!(derive("frase longa e secreta", &wrapped).is_err(), "custo que travaria o app");
        assert!(DEFAULT_COST.is_sane());
    }

    #[test]
    fn short_password_is_refused() {
        assert!(matches!(new_kek_with("curta", TEST_COST), Err(AppError::WeakPassword)));
    }

    #[test]
    fn token_is_bound_to_its_account() {
        let kek = new_kek_with("frase longa e secreta", TEST_COST).unwrap();
        let sealed = seal_token(&kek, "conta-a", "1//token-de-teste").unwrap();
        assert!(!sealed.contains("token-de-teste"));
        assert_eq!(&*open_token(&kek, "conta-a", &sealed).unwrap(), "1//token-de-teste");
        assert!(open_token(&kek, "conta-b", &sealed).is_err(), "o token de uma conta não serve para outra");
    }

    #[test]
    fn waits_grow_and_are_capped() {
        assert_eq!(wait_after(0), Duration::ZERO);
        assert_eq!(wait_after(1), Duration::from_secs(2));
        assert_eq!(wait_after(3), Duration::from_secs(8));
        assert_eq!(wait_after(20), MAX_WAIT);
        let now = Instant::now();
        let mut a = Attempts::default();
        assert_eq!(a.wait_seconds(now), 0);
        a.failed(now);
        a.failed(now);
        assert_eq!(a.wait_seconds(now), 4);
        a.succeeded();
        assert_eq!(a.wait_seconds(now), 0);
    }

    #[test]
    fn kek_never_shows_in_debug() {
        let kek = new_kek_with("frase longa e secreta", TEST_COST).unwrap();
        assert_eq!(format!("{kek:?}"), "Kek(…)");
    }
}
