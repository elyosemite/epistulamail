//! Chaveiro OpenPGP e configurações de criptografia no banco (US13). O banco é cifrado (US3, US10);
//! a parte secreta das minhas chaves fica sem senha própria aqui dentro.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::sql_error;
use crate::error::AppError;
use crate::openpgp::KeyInfo;
use crate::store::now;

/// Aceitação da chave de um contato (CA3, no modelo do Thunderbird).
pub const ACCEPTANCE: &[&str] = &["undecided", "rejected", "unverified", "verified"];

/// Uma chave como o gerenciador a mostra.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PgpKeyRow {
    pub fingerprint: String,
    pub user_ids: Vec<String>,
    pub emails: Vec<String>,
    pub algorithm: String,
    pub version: u8,
    pub post_quantum: bool,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub revoked: bool,
    pub acceptance: String,
    pub origin: String,
    pub added_at: i64,
    /// Tem a parte secreta (é minha).
    pub mine: bool,
}

const KEY_SELECT: &str = "SELECT fingerprint, user_ids, algorithm, version, post_quantum, created_at, expires_at, revoked, acceptance, origin, added_at,
        secret_key IS NOT NULL, (SELECT json_group_array(email) FROM pgp_key_email e WHERE e.fingerprint = k.fingerprint)
     FROM pgp_key k";

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<PgpKeyRow> {
    let uids: String = r.get(1)?;
    let emails: String = r.get(12)?;
    Ok(PgpKeyRow {
        fingerprint: r.get(0)?,
        user_ids: serde_json::from_str(&uids).unwrap_or_default(),
        algorithm: r.get(2)?,
        version: r.get(3)?,
        post_quantum: r.get(4)?,
        created_at: r.get(5)?,
        expires_at: r.get(6)?,
        revoked: r.get(7)?,
        acceptance: r.get(8)?,
        origin: r.get(9)?,
        added_at: r.get(10)?,
        mine: r.get(11)?,
        emails: serde_json::from_str(&emails).unwrap_or_default(),
    })
}

/// Grava uma chave nova ou atualiza uma conhecida. A aceitação de uma chave conhecida não muda; a
/// parte secreta só é acrescentada, nunca apagada por uma importação da pública.
pub fn upsert_key(conn: &mut Connection, info: &KeyInfo, public: &[u8], secret: Option<&[u8]>, origin: &str) -> Result<PgpKeyRow, AppError> {
    let tx = conn.transaction().map_err(sql_error("chaveiro"))?;
    // Minha chave nasce aceita e conferida; a de um contato, "ainda não".
    let acceptance = if secret.is_some() { "verified" } else { "undecided" };
    tx.execute(
        "INSERT INTO pgp_key (fingerprint, public_key, secret_key, user_ids, algorithm, version, post_quantum, created_at, expires_at, revoked, acceptance, origin, added_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT (fingerprint) DO UPDATE SET public_key = excluded.public_key, secret_key = coalesce(excluded.secret_key, pgp_key.secret_key),
            user_ids = excluded.user_ids, algorithm = excluded.algorithm, expires_at = excluded.expires_at, revoked = excluded.revoked,
            acceptance = CASE WHEN excluded.secret_key IS NOT NULL THEN 'verified' ELSE pgp_key.acceptance END",
        params![
            info.fingerprint,
            public,
            secret,
            serde_json::to_string(&info.user_ids).unwrap_or_else(|_| "[]".into()),
            info.algorithm,
            info.version,
            info.post_quantum,
            info.created_at,
            info.expires_at,
            info.revoked,
            acceptance,
            origin,
            now(),
        ],
    )
    .map_err(sql_error("chaveiro"))?;
    tx.execute("DELETE FROM pgp_key_email WHERE fingerprint = ?1", [&info.fingerprint]).map_err(sql_error("chaveiro"))?;
    for email in &info.emails {
        tx.execute("INSERT OR IGNORE INTO pgp_key_email (email, fingerprint) VALUES (?1, ?2)", params![email, info.fingerprint]).map_err(sql_error("chaveiro"))?;
    }
    tx.commit().map_err(sql_error("chaveiro"))?;
    get_key(conn, &info.fingerprint)?.ok_or(AppError::Storage)
}

pub fn get_key(conn: &Connection, fingerprint: &str) -> Result<Option<PgpKeyRow>, AppError> {
    conn.query_row(&format!("{KEY_SELECT} WHERE k.fingerprint = ?1"), [fingerprint], row).optional().map_err(sql_error("chaveiro"))
}

pub fn list_keys(conn: &Connection) -> Result<Vec<PgpKeyRow>, AppError> {
    let mut st = conn.prepare_cached(&format!("{KEY_SELECT} ORDER BY secret_key IS NULL, lower(user_ids)")).map_err(sql_error("chaveiro"))?;
    let rows = st.query_map([], row).map_err(sql_error("chaveiro"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("chaveiro"))
}

pub fn public_key(conn: &Connection, fingerprint: &str) -> Result<Option<Vec<u8>>, AppError> {
    conn.query_row("SELECT public_key FROM pgp_key WHERE fingerprint = ?1", [fingerprint], |r| r.get(0)).optional().map_err(sql_error("chaveiro"))
}

pub fn secret_key(conn: &Connection, fingerprint: &str) -> Result<Option<Vec<u8>>, AppError> {
    conn.query_row("SELECT secret_key FROM pgp_key WHERE fingerprint = ?1", [fingerprint], |r| r.get::<_, Option<Vec<u8>>>(0))
        .optional()
        .map(Option::flatten)
        .map_err(sql_error("chaveiro"))
}

/// Todas as minhas chaves secretas (para decifrar o que chegar para qualquer uma delas).
pub fn all_secret_keys(conn: &Connection) -> Result<Vec<Vec<u8>>, AppError> {
    let mut st = conn.prepare_cached("SELECT secret_key FROM pgp_key WHERE secret_key IS NOT NULL").map_err(sql_error("chaveiro"))?;
    let rows = st.query_map([], |r| r.get(0)).map_err(sql_error("chaveiro"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("chaveiro"))
}

/// Chave pública de um e-mail que serve para cifrar: aceita, não rejeitada, não revogada nem vencida.
/// Entre várias, a mais nova.
pub fn usable_key_for(conn: &Connection, email: &str) -> Result<Option<(String, Vec<u8>)>, AppError> {
    conn.query_row(
        "SELECT k.fingerprint, k.public_key FROM pgp_key k JOIN pgp_key_email e ON e.fingerprint = k.fingerprint
         WHERE e.email = ?1 AND k.revoked = 0 AND (k.expires_at IS NULL OR k.expires_at > ?2)
           AND (k.secret_key IS NOT NULL OR k.acceptance IN ('unverified', 'verified'))
         ORDER BY k.created_at DESC LIMIT 1",
        params![email.trim().to_lowercase(), now()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
    .map_err(sql_error("chaveiro"))
}

/// Chaves públicas de um e-mail, qualquer aceitação (para verificar assinaturas).
pub fn keys_for(conn: &Connection, email: &str) -> Result<Vec<Vec<u8>>, AppError> {
    let mut st = conn
        .prepare_cached("SELECT k.public_key FROM pgp_key k JOIN pgp_key_email e ON e.fingerprint = k.fingerprint WHERE e.email = ?1 AND k.acceptance <> 'rejected'")
        .map_err(sql_error("chaveiro"))?;
    let rows = st.query_map([email.trim().to_lowercase()], |r| r.get(0)).map_err(sql_error("chaveiro"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("chaveiro"))
}

pub fn set_acceptance(conn: &Connection, fingerprint: &str, acceptance: &str) -> Result<PgpKeyRow, AppError> {
    if !ACCEPTANCE.contains(&acceptance) {
        return Err(AppError::InvalidInput);
    }
    let changed = conn
        .execute("UPDATE pgp_key SET acceptance = ?2 WHERE fingerprint = ?1 AND secret_key IS NULL", params![fingerprint, acceptance])
        .map_err(sql_error("chaveiro"))?;
    if changed == 0 {
        return Err(AppError::InvalidId);
    }
    get_key(conn, fingerprint)?.ok_or(AppError::InvalidId)
}

pub fn delete_key(conn: &Connection, fingerprint: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM pgp_key WHERE fingerprint = ?1", [fingerprint]).map_err(sql_error("chaveiro"))?;
    Ok(())
}

// ---------------------------------------------------------------- configurações por conta (CA1)

/// Configurações → Criptografia de uma conta. Padrões das decisões D3 e D4 da US13.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CryptoSettings {
    /// Impressão digital da minha chave usada por esta conta.
    pub pgp_key: Option<String>,
    /// Mensagens novas começam criptografadas.
    pub encrypt_new: bool,
    /// Ligar o cadeado sozinho quando todos os destinatários tiverem chave.
    pub auto_encrypt: bool,
    pub sign_unencrypted: bool,
    pub attach_key: bool,
    pub autocrypt: bool,
    pub encrypt_subject: bool,
    /// "auto", "openpgp" ou "smime".
    pub technology: String,
    /// Buscar chaves na internet: "ask", "always" ou "never" (D5).
    pub search_online: String,
}

impl Default for CryptoSettings {
    fn default() -> Self {
        CryptoSettings {
            pgp_key: None,
            encrypt_new: false,
            auto_encrypt: true,
            sign_unencrypted: false,
            attach_key: true,
            autocrypt: true,
            encrypt_subject: true,
            technology: "auto".into(),
            search_online: "ask".into(),
        }
    }
}

pub fn crypto_settings(conn: &Connection, account_id: &str) -> Result<CryptoSettings, AppError> {
    let json: Option<String> = conn
        .query_row("SELECT settings FROM account_crypto WHERE account_id = ?1", [account_id], |r| r.get(0))
        .optional()
        .map_err(sql_error("criptografia"))?;
    Ok(json.and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default())
}

pub fn set_crypto_settings(conn: &Connection, account_id: &str, settings: &CryptoSettings) -> Result<CryptoSettings, AppError> {
    let valid = ["auto", "openpgp", "smime"].contains(&settings.technology.as_str()) && ["ask", "always", "never"].contains(&settings.search_online.as_str());
    if !valid {
        return Err(AppError::InvalidInput);
    }
    if let Some(fp) = &settings.pgp_key {
        let mine: bool = conn
            .query_row("SELECT secret_key IS NOT NULL FROM pgp_key WHERE fingerprint = ?1", [fp], |r| r.get(0))
            .optional()
            .map_err(sql_error("criptografia"))?
            .unwrap_or(false);
        if !mine {
            return Err(AppError::InvalidInput);
        }
    }
    let json = serde_json::to_string(settings).map_err(|_| AppError::InvalidInput)?;
    conn.execute(
        "INSERT INTO account_crypto (account_id, settings) VALUES (?1, ?2) ON CONFLICT (account_id) DO UPDATE SET settings = excluded.settings",
        params![account_id, json],
    )
    .map_err(sql_error("criptografia"))?;
    crypto_settings(conn, account_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openpgp::{self, KeyKind};
    use crate::store::tests::{account, test_db};

    #[test]
    fn keys_are_stored_found_by_email_and_accepted() {
        let t = test_db();
        t.db.with(|c| {
            let mine = openpgp::generate(KeyKind::Curve25519, "Eu", "eu@exemplo.com").unwrap();
            let info = openpgp::info(&mine.to_public_key());
            let pubb = openpgp::public_bytes(&mine.to_public_key()).unwrap();
            let row = upsert_key(c, &info, &pubb, Some(&openpgp::secret_bytes(&mine).unwrap()), "generated").unwrap();
            assert!(row.mine);
            assert_eq!(row.acceptance, "verified");
            assert_eq!(all_secret_keys(c).unwrap().len(), 1);

            let ana = openpgp::generate(KeyKind::Curve25519, "Ana", "Ana@Lumen.com.br").unwrap().to_public_key();
            let ana_info = openpgp::info(&ana);
            upsert_key(c, &ana_info, &openpgp::public_bytes(&ana).unwrap(), None, "autocrypt").unwrap();
            assert!(usable_key_for(c, "ana@lumen.com.br").unwrap().is_none(), "ainda não aceita não serve para cifrar");
            assert_eq!(keys_for(c, "ANA@lumen.com.br").unwrap().len(), 1, "mas serve para verificar");
            set_acceptance(c, &ana_info.fingerprint, "unverified").unwrap();
            assert_eq!(usable_key_for(c, "ana@lumen.com.br").unwrap().map(|(fp, _)| fp), Some(ana_info.fingerprint.clone()));
            assert!(set_acceptance(c, &info.fingerprint, "rejected").is_err(), "minha chave não tem aceitação");

            // Importar de novo a pública não muda a aceitação.
            upsert_key(c, &ana_info, &openpgp::public_bytes(&ana).unwrap(), None, "imported").unwrap();
            assert_eq!(get_key(c, &ana_info.fingerprint).unwrap().unwrap().acceptance, "unverified");
            assert_eq!(list_keys(c).unwrap().len(), 2);
            delete_key(c, &ana_info.fingerprint).unwrap();
            assert!(keys_for(c, "ana@lumen.com.br").unwrap().is_empty());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn crypto_settings_default_and_validate() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let s = crypto_settings(c, &a.id).unwrap();
            assert!(!s.encrypt_new && s.auto_encrypt && s.attach_key && s.autocrypt && s.encrypt_subject && !s.sign_unencrypted);
            assert_eq!((s.technology.as_str(), s.search_online.as_str()), ("auto", "ask"));
            let bad = CryptoSettings { technology: "x".into(), ..s.clone() };
            assert!(set_crypto_settings(c, &a.id, &bad).is_err());
            let foreign = CryptoSettings { pgp_key: Some("ABCD".into()), ..s.clone() };
            assert!(set_crypto_settings(c, &a.id, &foreign).is_err(), "só uma chave minha");
            let on = CryptoSettings { encrypt_new: true, ..s };
            assert!(set_crypto_settings(c, &a.id, &on).unwrap().encrypt_new);
            Ok(())
        })
        .unwrap();
    }
}
