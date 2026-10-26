//! Repositório do banco local (US3, CA2). Consultas sempre parametrizadas. Nenhuma função registra
//! em log endereço, nome de conta, nome de pasta, assunto, corpo ou nome de anexo.

use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;

use crate::db::sql_error;
use crate::error::AppError;
use crate::mailbox::{RemoteFolder, Role};
use crate::message::{AttachmentInfo, HeaderFields, MessageFlags};
use crate::providers::CatalogEntry;

pub const COLORS: &[&str] = &["blue", "teal", "green", "purple", "magenta", "orange"];
pub const MAX_NAME_CHARS: usize = 32;
const SETTING_LAST_FOLDER: &str = "last_folder";
const SETTING_LIST_WIDTH: &str = "list_width";
const SETTING_SYNC_MINUTES: &str = "sync_minutes";
const APP_SYNC_DEFAULT: &str = "sync_default_minutes";
/// Intervalos de sincronização aceitos, em minutos (US5, D3). `None` é "Manualmente".
pub const SYNC_INTERVALS: &[u32] = &[1, 2, 5, 10, 15, 30, 60];
pub const SYNC_DEFAULT_MINUTES: u32 = 5;
/// Limites da largura da lista em pixels (20rem a 35rem com fonte de 16px).
pub const LIST_WIDTH_RANGE: std::ops::RangeInclusive<u32> = 320..=560;
/// Mensagem movida fica protegida da limpeza de órfãs por este tempo (segundos).
const IN_TRANSIT_SECS: i64 = 600;

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Novo id de conta: UUID v4 a partir do gerador do SO.
pub fn new_account_id() -> String {
    use rand::RngCore;
    let mut b = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

// ---------------------------------------------------------------- contas

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Active,
    NeedsReauth,
    SignedOut,
}

impl AccountStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            AccountStatus::Active => "active",
            AccountStatus::NeedsReauth => "needs_reauth",
            AccountStatus::SignedOut => "signed_out",
        }
    }

    fn parse(value: &str) -> AccountStatus {
        match value {
            "active" => AccountStatus::Active,
            "needs_reauth" => AccountStatus::NeedsReauth,
            _ => AccountStatus::SignedOut,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub provider: String,
    pub email: String,
    pub display_name: String,
    pub color: String,
    pub position: i64,
    pub status: AccountStatus,
    /// Não lidas da Caixa de entrada, para o contador da coluna de contas.
    pub unread: Option<u32>,
    pub last_folder_id: Option<String>,
    /// Largura da lista em pixels (divisor entre lista e leitura), por conta.
    pub list_width: Option<u32>,
    /// Foto do perfil do provedor como `data:` URL (a WebView não busca nada no provedor).
    pub photo: Option<String>,
}

const ACCOUNT_SELECT: &str = "SELECT a.id, a.provider, a.email, a.display_name, a.color, a.position, a.status,
        (SELECT f.unread FROM folder f WHERE f.account_id = a.id AND f.role = 'inbox' LIMIT 1),
        (SELECT s.value FROM account_setting s WHERE s.account_id = a.id AND s.key = 'last_folder'),
        (SELECT s.value FROM account_setting s WHERE s.account_id = a.id AND s.key = 'list_width'),
        a.photo, a.photo_type
    FROM account a";

fn account_from(row: &Row<'_>) -> rusqlite::Result<Account> {
    Ok(Account {
        id: row.get(0)?,
        provider: row.get(1)?,
        email: row.get(2)?,
        display_name: row.get(3)?,
        color: row.get(4)?,
        position: row.get(5)?,
        status: AccountStatus::parse(&row.get::<_, String>(6)?),
        unread: row.get::<_, Option<i64>>(7)?.and_then(|n| u32::try_from(n).ok()),
        last_folder_id: row.get(8)?,
        list_width: row.get::<_, Option<String>>(9)?.and_then(|v| v.parse().ok()),
        photo: match (row.get::<_, Option<Vec<u8>>>(10)?, row.get::<_, Option<String>>(11)?) {
            (Some(bytes), Some(kind)) => Some(photo_data_url(&kind, &bytes)),
            _ => None,
        },
    })
}

pub fn list_accounts(conn: &Connection) -> Result<Vec<Account>, AppError> {
    let mut st = conn.prepare_cached(&format!("{ACCOUNT_SELECT} ORDER BY a.position, a.created_at")).map_err(sql_error("contas"))?;
    let rows = st.query_map([], account_from).map_err(sql_error("contas"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("contas"))
}

pub fn get_account(conn: &Connection, id: &str) -> Result<Option<Account>, AppError> {
    conn.query_row(&format!("{ACCOUNT_SELECT} WHERE a.id = ?1"), [id], account_from)
        .optional()
        .map_err(sql_error("conta"))
}

pub fn require_account(conn: &Connection, id: &str) -> Result<Account, AppError> {
    get_account(conn, id)?.ok_or(AppError::AccountNotFound)
}

pub fn find_account(conn: &Connection, provider: &str, email: &str) -> Result<Option<Account>, AppError> {
    conn.query_row(&format!("{ACCOUNT_SELECT} WHERE a.provider = ?1 AND a.email = ?2"), [provider, &email.to_lowercase()], account_from)
        .optional()
        .map_err(sql_error("conta"))
}

/// Nome sugerido: a parte do e-mail antes do "@", limitada a 32 caracteres.
pub fn default_display_name(email: &str) -> String {
    let local = email.split('@').next().unwrap_or_default().trim();
    let name: String = local.chars().take(MAX_NAME_CHARS).collect();
    if name.is_empty() {
        "Conta".to_owned()
    } else {
        name
    }
}

/// Primeira cor ainda não usada; com todas usadas, repete na ordem.
pub fn next_color(conn: &Connection) -> Result<&'static str, AppError> {
    let mut st = conn.prepare_cached("SELECT color FROM account").map_err(sql_error("cores"))?;
    let used: Vec<String> = st.query_map([], |r| r.get(0)).map_err(sql_error("cores"))?.collect::<Result<_, _>>().map_err(sql_error("cores"))?;
    Ok(COLORS
        .iter()
        .find(|c| !used.iter().any(|u| u == *c))
        .copied()
        .unwrap_or(COLORS[used.len() % COLORS.len()]))
}

pub fn validate_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    let count = name.chars().count();
    if count == 0 || count > MAX_NAME_CHARS || name.chars().any(char::is_control) {
        return Err(AppError::InvalidInput);
    }
    Ok(name.to_owned())
}

pub fn validate_color(color: &str) -> Result<&'static str, AppError> {
    COLORS.iter().find(|c| **c == color).copied().ok_or(AppError::InvalidInput)
}

pub fn insert_account(conn: &mut Connection, id: &str, entry: &CatalogEntry, email: &str, display_name: &str, color: &str) -> Result<Account, AppError> {
    let (Some(imap), Some(smtp)) = (entry.imap, entry.smtp) else { return Err(AppError::ProviderUnavailable) };
    let display_name = validate_name(display_name)?;
    let color = validate_color(color)?;
    let tx = conn.transaction().map_err(sql_error("nova conta"))?;
    tx.execute(
        "INSERT INTO account (id, provider, email, display_name, color, position, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, (SELECT coalesce(max(position) + 1, 0) FROM account), 'active', ?6)",
        params![id, entry.id, email.to_lowercase(), display_name, color, now()],
    )
    .map_err(sql_error("nova conta"))?;
    tx.execute(
        "INSERT INTO provider_config (account_id, auth_kind, imap_host, imap_port, imap_security, smtp_host, smtp_port, smtp_security)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, entry.auth.as_str(), imap.host, imap.port, imap.security.as_str(), smtp.host, smtp.port, smtp.security.as_str()],
    )
    .map_err(sql_error("nova conta"))?;
    tx.commit().map_err(sql_error("nova conta"))?;
    tracing::info!(provider = entry.id, "conta criada no banco");
    require_account(conn, id)
}

pub fn update_account(conn: &Connection, id: &str, display_name: Option<&str>, color: Option<&str>) -> Result<Account, AppError> {
    require_account(conn, id)?;
    if let Some(name) = display_name {
        let name = validate_name(name)?;
        conn.execute("UPDATE account SET display_name = ?2 WHERE id = ?1", params![id, name]).map_err(sql_error("renomear conta"))?;
    }
    if let Some(color) = color {
        let color = validate_color(color)?;
        conn.execute("UPDATE account SET color = ?2 WHERE id = ?1", params![id, color]).map_err(sql_error("cor da conta"))?;
    }
    require_account(conn, id)
}

/// Move a conta uma posição para cima (`-1`) ou para baixo (`1`) e renumera todas.
pub fn move_account(conn: &mut Connection, id: &str, delta: i64) -> Result<Vec<Account>, AppError> {
    let mut ids: Vec<String> = list_accounts(conn)?.into_iter().map(|a| a.id).collect();
    let from = ids.iter().position(|a| a == id).ok_or(AppError::AccountNotFound)?;
    let to = (from as i64 + delta).clamp(0, ids.len() as i64 - 1) as usize;
    let moved = ids.remove(from);
    ids.insert(to, moved);
    let tx = conn.transaction().map_err(sql_error("ordem das contas"))?;
    for (position, account) in ids.iter().enumerate() {
        tx.execute("UPDATE account SET position = ?2 WHERE id = ?1", params![account, position as i64]).map_err(sql_error("ordem das contas"))?;
    }
    tx.commit().map_err(sql_error("ordem das contas"))?;
    list_accounts(conn)
}

pub fn set_status(conn: &Connection, id: &str, status: AccountStatus) -> Result<(), AppError> {
    let changed = conn
        .execute("UPDATE account SET status = ?2 WHERE id = ?1", params![id, status.as_str()])
        .map_err(sql_error("status da conta"))?;
    if changed == 0 {
        return Err(AppError::AccountNotFound);
    }
    tracing::info!(status = status.as_str(), "status da conta alterado");
    Ok(())
}

pub fn set_last_folder(conn: &Connection, account_id: &str, folder_id: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO account_setting (account_id, key, value) VALUES (?1, ?2, ?3)
         ON CONFLICT (account_id, key) DO UPDATE SET value = excluded.value",
        params![account_id, SETTING_LAST_FOLDER, folder_id],
    )
    .map_err(sql_error("ajuste da conta"))?;
    Ok(())
}

pub fn set_list_width(conn: &Connection, account_id: &str, width: u32) -> Result<(), AppError> {
    if !LIST_WIDTH_RANGE.contains(&width) {
        return Err(AppError::InvalidInput);
    }
    require_account(conn, account_id)?;
    conn.execute(
        "INSERT INTO account_setting (account_id, key, value) VALUES (?1, ?2, ?3)
         ON CONFLICT (account_id, key) DO UPDATE SET value = excluded.value",
        params![account_id, SETTING_LIST_WIDTH, width.to_string()],
    )
    .map_err(sql_error("ajuste da conta"))?;
    Ok(())
}

// ---------------------------------------------------------------- foto da conta (US5, CA5)

fn photo_data_url(kind: &str, bytes: &[u8]) -> String {
    use base64::Engine;
    format!("data:{kind};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// Grava a foto baixada (ou apaga, com `None`) e marca a hora da verificação.
pub fn set_photo(conn: &Connection, account_id: &str, photo: Option<(&[u8], &str)>) -> Result<(), AppError> {
    let (bytes, kind) = photo.map_or((None, None), |(b, k)| (Some(b), Some(k)));
    let changed = conn
        .execute(
            "UPDATE account SET photo = ?2, photo_type = ?3, photo_checked_at = ?4 WHERE id = ?1",
            params![account_id, bytes, kind, now()],
        )
        .map_err(sql_error("foto da conta"))?;
    if changed == 0 {
        return Err(AppError::AccountNotFound);
    }
    Ok(())
}

/// A foto é verificada no máximo uma vez por dia (fora conectar e reconectar).
pub fn photo_due(conn: &Connection, account_id: &str) -> Result<bool, AppError> {
    let checked: Option<i64> = conn
        .query_row("SELECT photo_checked_at FROM account WHERE id = ?1", [account_id], |r| r.get(0))
        .optional()
        .map_err(sql_error("foto da conta"))?
        .ok_or(AppError::AccountNotFound)?;
    Ok(checked.is_none_or(|t| now() - t >= 24 * 3600))
}

// ---------------------------------------------------------------- intervalo de sincronização (US5, CA7)

/// `None` = manualmente. Só os valores de `SYNC_INTERVALS` são aceitos.
pub fn validate_interval(minutes: Option<u32>) -> Result<Option<u32>, AppError> {
    match minutes {
        Some(m) if !SYNC_INTERVALS.contains(&m) => Err(AppError::InvalidInput),
        other => Ok(other),
    }
}

fn encode_interval(minutes: Option<u32>) -> String {
    minutes.map_or_else(|| "manual".to_owned(), |m| m.to_string())
}

fn decode_interval(value: &str) -> Option<Option<u32>> {
    if value == "manual" {
        return Some(None);
    }
    value.parse().ok().filter(|m| SYNC_INTERVALS.contains(m)).map(Some)
}

pub fn default_interval(conn: &Connection) -> Result<Option<u32>, AppError> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM app_setting WHERE key = ?1", [APP_SYNC_DEFAULT], |r| r.get(0))
        .optional()
        .map_err(sql_error("ajuste do app"))?;
    Ok(value.as_deref().and_then(decode_interval).unwrap_or(Some(SYNC_DEFAULT_MINUTES)))
}

pub fn set_default_interval(conn: &Connection, minutes: Option<u32>) -> Result<(), AppError> {
    let minutes = validate_interval(minutes)?;
    conn.execute(
        "INSERT INTO app_setting (key, value) VALUES (?1, ?2) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![APP_SYNC_DEFAULT, encode_interval(minutes)],
    )
    .map_err(sql_error("ajuste do app"))?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSync {
    pub account_id: String,
    /// Intervalo que vale para a conta (o próprio ou o padrão). `None` = manualmente.
    pub minutes: Option<u32>,
    pub uses_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSettings {
    pub default_minutes: Option<u32>,
    pub accounts: Vec<AccountSync>,
}

pub fn sync_settings(conn: &Connection) -> Result<SyncSettings, AppError> {
    let default_minutes = default_interval(conn)?;
    let mut st = conn
        .prepare_cached(
            "SELECT a.id, (SELECT s.value FROM account_setting s WHERE s.account_id = a.id AND s.key = ?1)
             FROM account a ORDER BY a.position, a.created_at",
        )
        .map_err(sql_error("sincronização"))?;
    let rows = st
        .query_map([SETTING_SYNC_MINUTES], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)))
        .map_err(sql_error("sincronização"))?;
    let mut accounts = Vec::new();
    for row in rows {
        let (account_id, own) = row.map_err(sql_error("sincronização"))?;
        let own = own.as_deref().and_then(decode_interval);
        accounts.push(AccountSync { account_id, minutes: own.unwrap_or(default_minutes), uses_default: own.is_none() });
    }
    Ok(SyncSettings { default_minutes, accounts })
}

/// Aplica o mesmo intervalo a uma ou várias contas; `use_default` volta a seguir o padrão.
pub fn set_account_interval(conn: &mut Connection, account_ids: &[String], minutes: Option<u32>, use_default: bool) -> Result<(), AppError> {
    let minutes = validate_interval(minutes)?;
    if account_ids.is_empty() {
        return Err(AppError::InvalidInput);
    }
    let tx = conn.transaction().map_err(sql_error("sincronização"))?;
    for id in account_ids {
        require_account(&tx, id)?;
        if use_default {
            tx.execute("DELETE FROM account_setting WHERE account_id = ?1 AND key = ?2", params![id, SETTING_SYNC_MINUTES])
                .map_err(sql_error("sincronização"))?;
        } else {
            tx.execute(
                "INSERT INTO account_setting (account_id, key, value) VALUES (?1, ?2, ?3)
                 ON CONFLICT (account_id, key) DO UPDATE SET value = excluded.value",
                params![id, SETTING_SYNC_MINUTES, encode_interval(minutes)],
            )
            .map_err(sql_error("sincronização"))?;
        }
    }
    tx.commit().map_err(sql_error("sincronização"))?;
    tracing::info!(accounts = account_ids.len(), use_default, "intervalo de sincronização alterado");
    Ok(())
}

pub fn imap_server(conn: &Connection, account_id: &str) -> Result<(String, u16), AppError> {
    conn.query_row("SELECT imap_host, imap_port FROM provider_config WHERE account_id = ?1", [account_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()
        .map_err(sql_error("servidor da conta"))?
        .ok_or(AppError::AccountNotFound)
}

// ---------------------------------------------------------------- pastas

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderRow {
    pub id: i64,
    pub account_id: String,
    pub imap_name: String,
    pub role: Role,
    pub display_name: String,
    pub uidvalidity: Option<u32>,
    pub uidnext: Option<u32>,
    pub highestmodseq: Option<u64>,
    pub oldest_uid: Option<u32>,
    pub complete: bool,
    pub total: Option<u32>,
    pub unread: Option<u32>,
    pub last_sync_at: Option<i64>,
}

const FOLDER_SELECT: &str = "SELECT id, account_id, imap_name, role, display_name, uidvalidity, uidnext, highestmodseq,
    oldest_uid, complete, total, unread, last_sync_at FROM folder";

fn opt_u32(v: Option<i64>) -> Option<u32> {
    v.and_then(|n| u32::try_from(n).ok())
}

fn folder_from(row: &Row<'_>) -> rusqlite::Result<FolderRow> {
    Ok(FolderRow {
        id: row.get(0)?,
        account_id: row.get(1)?,
        imap_name: row.get(2)?,
        role: Role::parse(&row.get::<_, String>(3)?),
        display_name: row.get(4)?,
        uidvalidity: opt_u32(row.get(5)?),
        uidnext: opt_u32(row.get(6)?),
        highestmodseq: row.get::<_, Option<i64>>(7)?.and_then(|n| u64::try_from(n).ok()),
        oldest_uid: opt_u32(row.get(8)?),
        complete: row.get::<_, i64>(9)? != 0,
        total: opt_u32(row.get(10)?),
        unread: opt_u32(row.get(11)?),
        last_sync_at: row.get(12)?,
    })
}

pub fn list_folders(conn: &Connection, account_id: &str) -> Result<Vec<FolderRow>, AppError> {
    let mut st = conn.prepare_cached(&format!("{FOLDER_SELECT} WHERE account_id = ?1 ORDER BY position")).map_err(sql_error("pastas"))?;
    let rows = st.query_map([account_id], folder_from).map_err(sql_error("pastas"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("pastas"))
}

pub fn get_folder(conn: &Connection, id: i64) -> Result<Option<FolderRow>, AppError> {
    conn.query_row(&format!("{FOLDER_SELECT} WHERE id = ?1"), [id], folder_from)
        .optional()
        .map_err(sql_error("pasta"))
}

/// A pasta, desde que pertença à conta do id opaco. Um id de outra conta é `InvalidId`.
pub fn folder_of_account(conn: &Connection, account_id: &str, folder_id: i64) -> Result<FolderRow, AppError> {
    get_folder(conn, folder_id)?
        .filter(|f| f.account_id == account_id)
        .ok_or(AppError::InvalidId)
}

pub fn inbox_of(conn: &Connection, account_id: &str) -> Result<Option<FolderRow>, AppError> {
    conn.query_row(&format!("{FOLDER_SELECT} WHERE account_id = ?1 AND role = 'inbox' LIMIT 1"), [account_id], folder_from)
        .optional()
        .map_err(sql_error("pasta"))
}

/// Substitui a lista de pastas da conta pela do servidor: atualiza as existentes, cria as novas e
/// remove (com as ligações de mensagem) as que sumiram.
pub fn replace_folders(conn: &mut Connection, account_id: &str, remote: &[RemoteFolder]) -> Result<(), AppError> {
    let tx = conn.transaction().map_err(sql_error("pastas"))?;
    {
        let mut upsert = tx
            .prepare_cached(
                "INSERT INTO folder (account_id, imap_name, role, display_name, position, total, unread)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT (account_id, imap_name) DO UPDATE SET
                    role = excluded.role, display_name = excluded.display_name, position = excluded.position,
                    total = coalesce(excluded.total, folder.total), unread = coalesce(excluded.unread, folder.unread)",
            )
            .map_err(sql_error("pastas"))?;
        for (position, f) in remote.iter().enumerate() {
            upsert
                .execute(params![account_id, f.imap_name, f.role.as_str(), f.display_name, position as i64, f.total, f.unread])
                .map_err(sql_error("pastas"))?;
        }
        let names: HashSet<&str> = remote.iter().map(|f| f.imap_name.as_str()).collect();
        let mut existing = tx.prepare_cached("SELECT id, imap_name FROM folder WHERE account_id = ?1").map_err(sql_error("pastas"))?;
        let stale: Vec<i64> = existing
            .query_map([account_id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(sql_error("pastas"))?
            .filter_map(Result::ok)
            .filter(|(_, name)| !names.contains(name.as_str()))
            .map(|(id, _)| id)
            .collect();
        for id in &stale {
            tx.execute("DELETE FROM folder WHERE id = ?1", [id]).map_err(sql_error("pastas"))?;
        }
        if !stale.is_empty() {
            tracing::info!(removed = stale.len(), "pastas que sumiram do servidor foram removidas");
        }
        delete_orphans(&tx, account_id)?;
    }
    tx.commit().map_err(sql_error("pastas"))
}

// ---------------------------------------------------------------- mensagens

/// Cabeçalho vindo do servidor para gravar no banco.
#[derive(Debug, Clone)]
pub struct RemoteHeader {
    pub uid: u32,
    pub fields: HeaderFields,
    pub internal_date: Option<String>,
    pub size: u32,
    pub flags: MessageFlags,
}

/// Chave que identifica a mesma mensagem em várias pastas da conta (marcadores do Gmail):
/// Message-ID + tamanho. Sem Message-ID, a mensagem fica restrita à pasta e ao UID.
pub fn dedupe_key(folder: &FolderRow, uidvalidity: u32, header: &RemoteHeader) -> String {
    match &header.fields.message_id {
        Some(id) => format!("mid:{id}:{}", header.size),
        None => uid_key(folder, uidvalidity, header.uid),
    }
}

fn uid_key(folder: &FolderRow, uidvalidity: u32, uid: u32) -> String {
    format!("uid:{}:{uidvalidity}:{uid}", folder.id)
}

/// Retrato de uma pasta no servidor, montado por `sync` e aplicado aqui numa transação só.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// UIDVALIDITY mudou ou a pasta nunca foi sincronizada: descarta as ligações antigas.
    pub reset: bool,
    pub uidvalidity: u32,
    pub uidnext: Option<u32>,
    pub highestmodseq: Option<u64>,
    pub total: u32,
    pub unread: Option<u32>,
    pub headers: Vec<RemoteHeader>,
    pub flag_updates: Vec<(u32, MessageFlags)>,
    /// `(menor UID da janela, UIDs que ainda existem no servidor a partir dele)`.
    pub present: Option<(u32, Vec<u32>)>,
    pub oldest_uid: Option<u32>,
    pub complete: Option<bool>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ApplyReport {
    pub upserted: usize,
    pub flags_changed: usize,
    pub removed: usize,
    /// Mensagens que não existiam no banco (para as notificações, US8).
    pub new_ids: Vec<i64>,
}

/// Conversa da mensagem (US8): a do pai, se ele já está no banco; senão a raiz dos cabeçalhos.
/// `''` quando a mensagem não tem nenhum id (fica sozinha).
fn thread_for(conn: &Connection, account_id: &str, fields: &HeaderFields) -> Result<String, AppError> {
    if let Some(parent) = &fields.in_reply_to {
        let found: Option<String> = conn
            .query_row(
                "SELECT thread_id FROM message WHERE account_id = ?1 AND msgid = ?2 AND thread_id IS NOT NULL AND thread_id <> '' LIMIT 1",
                params![account_id, parent],
                |r| r.get(0),
            )
            .optional()
            .map_err(sql_error("conversa"))?;
        if let Some(t) = found {
            return Ok(t);
        }
    }
    Ok(crate::message::thread_root(fields).unwrap_or_default())
}

/// `In-Reply-To` e `References`, separados por espaço, sem repetir.
fn refs_of(fields: &HeaderFields) -> String {
    let mut out: Vec<&str> = Vec::new();
    for id in fields.in_reply_to.iter().chain(fields.references.iter()) {
        if !out.contains(&id.as_str()) {
            out.push(id);
        }
    }
    out.join(" ")
}

pub fn apply_snapshot(conn: &mut Connection, folder_id: i64, snap: &Snapshot) -> Result<ApplyReport, AppError> {
    let folder = get_folder(conn, folder_id)?.ok_or(AppError::InvalidId)?;
    let tx = conn.transaction().map_err(sql_error("sincronização"))?;
    let mut report = ApplyReport::default();
    {
        if snap.reset {
            tx.execute("DELETE FROM message_folder WHERE folder_id = ?1", [folder_id]).map_err(sql_error("sincronização"))?;
        }
        let mut upsert = tx
            .prepare_cached(
                "INSERT INTO message (account_id, dedupe_key, subject, from_name, from_address, to_json, cc_json, date, size, seen, flagged, has_attachments, msgid, refs, thread_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                 ON CONFLICT (account_id, dedupe_key) DO UPDATE SET seen = excluded.seen, flagged = excluded.flagged, in_transit_since = NULL,
                    msgid = coalesce(message.msgid, excluded.msgid), refs = coalesce(message.refs, excluded.refs),
                    thread_id = coalesce(message.thread_id, excluded.thread_id)
                 RETURNING id",
            )
            .map_err(sql_error("sincronização"))?;
        let mut link = tx
            .prepare_cached(
                "INSERT INTO message_folder (message_id, folder_id, uid) VALUES (?1, ?2, ?3)
                 ON CONFLICT (folder_id, uid) DO UPDATE SET message_id = excluded.message_id",
            )
            .map_err(sql_error("sincronização"))?;
        // Duas cópias iguais na mesma pasta são mensagens distintas no servidor (cada UID tem suas
        // flags): só cópias em pastas diferentes viram uma mensagem só.
        let mut twin = tx
            .prepare_cached(
                "SELECT 1 FROM message m JOIN message_folder mf ON mf.message_id = m.id
                 WHERE m.account_id = ?1 AND m.dedupe_key = ?2 AND mf.folder_id = ?3 AND mf.uid <> ?4 LIMIT 1",
            )
            .map_err(sql_error("sincronização"))?;
        let mut exists = tx
            .prepare_cached("SELECT 1 FROM message WHERE account_id = ?1 AND dedupe_key = ?2")
            .map_err(sql_error("sincronização"))?;
        for h in &snap.headers {
            let mut key = dedupe_key(&folder, snap.uidvalidity, h);
            if twin.exists(params![folder.account_id, key, folder_id, h.uid]).map_err(sql_error("sincronização"))? {
                key = uid_key(&folder, snap.uidvalidity, h.uid);
            }
            let is_new = !exists.exists(params![folder.account_id, key]).map_err(sql_error("sincronização"))?;
            let thread = thread_for(&tx, &folder.account_id, &h.fields)?;
            let msgid = h.fields.message_id.as_deref().map(crate::message::normalize_id);
            let to = serde_json::to_string(&h.fields.to).unwrap_or_else(|_| "[]".into());
            let cc = serde_json::to_string(&h.fields.cc).unwrap_or_else(|_| "[]".into());
            let date = h.fields.date.clone().or_else(|| h.internal_date.clone()).unwrap_or_default();
            let id: i64 = upsert
                .query_row(
                    params![
                        folder.account_id,
                        key,
                        h.fields.subject,
                        h.fields.from_name,
                        h.fields.from_address,
                        to,
                        cc,
                        date,
                        h.size,
                        h.flags.seen,
                        h.flags.flagged,
                        h.fields.has_attachments,
                        msgid,
                        refs_of(&h.fields),
                        thread,
                    ],
                    |r| r.get(0),
                )
                .map_err(sql_error("sincronização"))?;
            link.execute(params![id, folder_id, h.uid]).map_err(sql_error("sincronização"))?;
            report.upserted += 1;
            if is_new && !snap.reset {
                report.new_ids.push(id);
            }
        }

        let mut flags = tx
            .prepare_cached(
                "UPDATE message SET seen = ?3, flagged = ?4
                 WHERE id = (SELECT message_id FROM message_folder WHERE folder_id = ?1 AND uid = ?2)",
            )
            .map_err(sql_error("sincronização"))?;
        for (uid, f) in &snap.flag_updates {
            report.flags_changed += flags.execute(params![folder_id, uid, f.seen, f.flagged]).map_err(sql_error("sincronização"))?;
        }

        if let Some((min, present)) = &snap.present {
            let present: HashSet<u32> = present.iter().copied().collect();
            let local = uids_from(&tx, folder_id, *min)?;
            let mut unlink = tx.prepare_cached("DELETE FROM message_folder WHERE folder_id = ?1 AND uid = ?2").map_err(sql_error("sincronização"))?;
            for uid in local.into_iter().filter(|u| !present.contains(u)) {
                report.removed += unlink.execute(params![folder_id, uid]).map_err(sql_error("sincronização"))?;
            }
        }
        delete_orphans(&tx, &folder.account_id)?;
        // Estrela tirada em outro dispositivo: o prazo local deixa de valer (concluídas ficam).
        tx.execute(
            "DELETE FROM message_followup WHERE completed_at IS NULL
                AND message_id IN (SELECT id FROM message WHERE account_id = ?1 AND flagged = 0)",
            [&folder.account_id],
        )
        .map_err(sql_error("sincronização"))?;

        let oldest = if snap.reset {
            snap.oldest_uid
        } else {
            match (folder.oldest_uid, snap.oldest_uid) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            }
        };
        let complete = snap.complete.unwrap_or(folder.complete && !snap.reset);
        tx.execute(
            "UPDATE folder SET uidvalidity = ?2, uidnext = ?3, highestmodseq = ?4, oldest_uid = ?5, complete = ?6,
                total = ?7, unread = coalesce(?8, unread), last_sync_at = ?9
             WHERE id = ?1",
            params![
                folder_id,
                snap.uidvalidity,
                snap.uidnext,
                snap.highestmodseq.and_then(|m| i64::try_from(m).ok()),
                oldest,
                complete,
                snap.total,
                snap.unread,
                now()
            ],
        )
        .map_err(sql_error("sincronização"))?;
        // Os contadores do servidor ainda contam as mensagens que estão saindo (ou não contam as que
        // estão chegando) por um mover pendente deste computador.
        tx.execute(
            "UPDATE folder SET
                total = max(0, coalesce(total, 0)
                    - (SELECT count(*) FROM pending_move pm WHERE pm.from_folder_id = ?1 AND pm.state <> 'moved')
                    + (SELECT count(*) FROM pending_move pm WHERE pm.to_folder_id = ?1
                        AND NOT EXISTS (SELECT 1 FROM message_folder x WHERE x.message_id = pm.message_id AND x.folder_id = ?1))),
                unread = CASE WHEN unread IS NULL THEN NULL ELSE max(0, unread
                    - (SELECT count(*) FROM pending_move pm JOIN message m ON m.id = pm.message_id
                        WHERE pm.from_folder_id = ?1 AND pm.state <> 'moved' AND m.seen = 0)
                    + (SELECT count(*) FROM pending_move pm JOIN message m ON m.id = pm.message_id WHERE pm.to_folder_id = ?1 AND m.seen = 0
                        AND NOT EXISTS (SELECT 1 FROM message_folder x WHERE x.message_id = pm.message_id AND x.folder_id = ?1))) END
             WHERE id = ?1",
            [folder_id],
        )
        .map_err(sql_error("sincronização"))?;
        tx.execute("UPDATE account SET last_sync_at = ?2 WHERE id = ?1", params![folder.account_id, now()])
            .map_err(sql_error("sincronização"))?;
    }
    tx.commit().map_err(sql_error("sincronização"))?;
    Ok(report)
}

fn uids_from(conn: &Connection, folder_id: i64, min: u32) -> Result<Vec<u32>, AppError> {
    let mut st = conn.prepare_cached("SELECT uid FROM message_folder WHERE folder_id = ?1 AND uid >= ?2").map_err(sql_error("uids"))?;
    let rows = st.query_map(params![folder_id, min], |r| r.get::<_, u32>(0)).map_err(sql_error("uids"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("uids"))
}

/// Mensagens sem nenhuma pasta saem do banco (com corpo e anexos, por cascade).
fn delete_orphans(conn: &Connection, account_id: &str) -> Result<usize, AppError> {
    conn.execute(
        "DELETE FROM message WHERE account_id = ?1
            AND NOT EXISTS (SELECT 1 FROM message_folder mf WHERE mf.message_id = message.id)
            AND (in_transit_since IS NULL OR in_transit_since < ?2)",
        params![account_id, now() - IN_TRANSIT_SECS],
    )
    .map_err(sql_error("órfãs"))
}

/// Sinalização com prazo (US4, CA7). `due` em `AAAA-MM-DD`; `None` é "sem data".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Followup {
    pub due: Option<String>,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedMessage {
    pub uid: u32,
    pub subject: String,
    pub from_name: String,
    pub from_address: String,
    pub date: String,
    pub flags: MessageFlags,
    pub followup: Option<Followup>,
    /// Trecho do texto (densidade Confortável); `None` enquanto a prévia não foi buscada.
    pub snippet: Option<String>,
    pub has_files: bool,
    pub has_images: bool,
    /// Conversa (US8): o `thread_id`, ou `#<id>` para a mensagem sozinha.
    pub thread_key: String,
    /// Resumo da conversa, só na lista da pasta.
    pub thread: Option<ThreadSummary>,
}

/// Uma linha por conversa na lista (US8, D1): quantas mensagens, quem participou e o estado geral.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSummary {
    pub count: u32,
    pub unread: bool,
    pub flagged: bool,
    /// Remetentes na ordem em que entraram na conversa, sem repetir.
    pub participants: Vec<Participant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub name: String,
    pub mine: bool,
}

fn listed_from(r: &Row<'_>) -> rusqlite::Result<ListedMessage> {
    let has_followup: bool = r.get(7)?;
    Ok(ListedMessage {
        uid: r.get(0)?,
        subject: r.get(1)?,
        from_name: r.get(2)?,
        from_address: r.get(3)?,
        date: r.get(4)?,
        flags: MessageFlags { seen: r.get(5)?, flagged: r.get(6)? },
        followup: has_followup.then(|| Followup { due: r.get(8).ok().flatten(), completed: r.get::<_, Option<i64>>(9).ok().flatten().is_some() }),
        snippet: r.get(10)?,
        has_files: r.get(11)?,
        has_images: r.get(12)?,
        thread_key: r.get(13)?,
        thread: None,
    })
}

/// Sem a prévia (estrutura ainda não lida), o clipe segue o palpite do cabeçalho (`multipart/mixed`).
const LISTED_COLUMNS: &str = "mf.uid, m.subject, m.from_name, m.from_address, m.date, m.seen, m.flagged,
    fu.message_id IS NOT NULL, fu.due_date, fu.completed_at,
    m.snippet, CASE WHEN m.snippet IS NULL THEN m.has_attachments ELSE m.has_files END, m.has_images,
    CASE WHEN m.thread_id IS NULL OR m.thread_id = '' THEN '#' || m.id ELSE m.thread_id END";

/// Há na mesma pasta uma mensagem mais nova da mesma conversa (e que não está saindo dela).
const NEWER_IN_THREAD: &str = "m.thread_id IS NOT NULL AND m.thread_id <> '' AND EXISTS (
        SELECT 1 FROM message m2 JOIN message_folder mf2 ON mf2.message_id = m2.id
        WHERE m2.account_id = m.account_id AND m2.thread_id = m.thread_id AND mf2.folder_id = mf.folder_id AND mf2.uid > mf.uid
          AND NOT EXISTS (SELECT 1 FROM pending_move pm2 WHERE pm2.from_folder_id = mf2.folder_id AND pm2.from_uid = mf2.uid))";

/// Resumo de cada conversa (fora Lixeira e Spam), para a lista. `own` é o e-mail da conta.
pub fn thread_summaries(conn: &Connection, account_id: &str, own: &str, keys: &[String]) -> Result<HashMap<String, ThreadSummary>, AppError> {
    let own = own.trim().to_lowercase();
    let mut st = conn
        .prepare_cached(
            "SELECT m.from_name, m.from_address, m.seen, m.flagged FROM message m
             WHERE m.account_id = ?1 AND m.thread_id = ?2
               AND EXISTS (SELECT 1 FROM message_folder mf JOIN folder f ON f.id = mf.folder_id
                           WHERE mf.message_id = m.id AND f.role NOT IN ('trash', 'spam'))
             ORDER BY m.date",
        )
        .map_err(sql_error("conversa"))?;
    let mut out = HashMap::new();
    for key in keys.iter().filter(|k| !k.starts_with('#')) {
        if out.contains_key(key) {
            continue;
        }
        let rows = st
            .query_map(params![account_id, key], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, bool>(2)?, r.get::<_, bool>(3)?)))
            .map_err(sql_error("conversa"))?;
        let mut summary = ThreadSummary { count: 0, unread: false, flagged: false, participants: Vec::new() };
        let mut seen_addresses: Vec<String> = Vec::new();
        for row in rows {
            let (name, address, seen, flagged) = row.map_err(sql_error("conversa"))?;
            summary.count += 1;
            summary.unread |= !seen;
            summary.flagged |= flagged;
            let address = address.trim().to_lowercase();
            if !seen_addresses.contains(&address) {
                summary.participants.push(Participant { name: if name.trim().is_empty() { address.clone() } else { name.trim().to_owned() }, mine: address == own });
                seen_addresses.push(address);
            }
        }
        if summary.count > 0 {
            out.insert(key.clone(), summary);
        }
    }
    Ok(out)
}

/// Página da pasta em ordem decrescente de UID, a partir de `before` (exclusivo).
pub fn list_page(conn: &Connection, folder_id: i64, before: Option<u32>, limit: u32) -> Result<Vec<ListedMessage>, AppError> {
    let mut st = conn
        .prepare_cached(
            &format!(
                "SELECT {LISTED_COLUMNS}
                 FROM message_folder mf JOIN message m ON m.id = mf.message_id
                 LEFT JOIN message_followup fu ON fu.message_id = m.id
                 WHERE mf.folder_id = ?1 AND mf.uid < ?2
                   AND NOT EXISTS (SELECT 1 FROM pending_move pm WHERE pm.from_folder_id = mf.folder_id AND pm.from_uid = mf.uid)
                   AND NOT ({NEWER_IN_THREAD})
                 ORDER BY mf.uid DESC LIMIT ?3"
            ),
        )
        .map_err(sql_error("lista"))?;
    let before = before.map(i64::from).unwrap_or(i64::MAX);
    let rows = st.query_map(params![folder_id, before, limit], listed_from).map_err(sql_error("lista"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("lista"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredMessage {
    pub id: i64,
    pub flags: MessageFlags,
    pub body_state: String,
}

pub fn message_by_uid(conn: &Connection, folder_id: i64, uid: u32) -> Result<Option<StoredMessage>, AppError> {
    conn.query_row(
        "SELECT m.id, m.seen, m.flagged, m.body_state FROM message_folder mf JOIN message m ON m.id = mf.message_id
         WHERE mf.folder_id = ?1 AND mf.uid = ?2",
        params![folder_id, uid],
        |r| Ok(StoredMessage { id: r.get(0)?, flags: MessageFlags { seen: r.get(1)?, flagged: r.get(2)? }, body_state: r.get(3)? }),
    )
    .optional()
    .map_err(sql_error("mensagem"))
}

pub fn load_body(conn: &Connection, message_id: i64) -> Result<Option<Vec<u8>>, AppError> {
    conn.query_row("SELECT raw FROM message_body WHERE message_id = ?1", [message_id], |r| r.get(0))
        .optional()
        .map_err(sql_error("corpo"))
}

pub fn save_body(conn: &mut Connection, message_id: i64, raw: &[u8], attachments: &[AttachmentInfo]) -> Result<(), AppError> {
    let tx = conn.transaction().map_err(sql_error("corpo"))?;
    tx.execute(
        "INSERT INTO message_body (message_id, raw, fetched_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (message_id) DO UPDATE SET raw = excluded.raw, fetched_at = excluded.fetched_at",
        params![message_id, raw, now()],
    )
    .map_err(sql_error("corpo"))?;
    tx.execute("DELETE FROM attachment WHERE message_id = ?1", [message_id]).map_err(sql_error("corpo"))?;
    for a in attachments {
        tx.execute(
            "INSERT INTO attachment (message_id, name, mime_type, size) VALUES (?1, ?2, ?3, ?4)",
            params![message_id, a.name, a.mime_type, a.size as i64],
        )
        .map_err(sql_error("corpo"))?;
    }
    let preview = crate::preview::from_raw(raw);
    tx.execute(
        "UPDATE message SET body_state = 'cached', has_attachments = ?2, snippet = ?3, has_files = ?4, has_images = ?5 WHERE id = ?1",
        params![message_id, !attachments.is_empty(), preview.snippet, preview.has_files, preview.has_images],
    )
    .map_err(sql_error("corpo"))?;
    tx.commit().map_err(sql_error("corpo"))
}

// ---------------------------------------------------------------- conversas (US8)

/// Uma mensagem da conversa: onde ela está (pasta e UID), a linha da lista e se é sua.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationItem {
    pub folder_id: i64,
    pub message_id: i64,
    pub message: ListedMessage,
    pub mine: bool,
}

/// Todas as mensagens da conversa de `message_id`, da mais antiga para a mais nova, uma vez cada,
/// na primeira pasta da sidebar em que estão. Lixeira e Spam ficam de fora (a não ser a própria).
pub fn conversation(conn: &Connection, account_id: &str, own: &str, message_id: i64) -> Result<Vec<ConversationItem>, AppError> {
    let thread: Option<String> = conn
        .query_row("SELECT thread_id FROM message WHERE id = ?1 AND account_id = ?2", params![message_id, account_id], |r| r.get(0))
        .optional()
        .map_err(sql_error("conversa"))?
        .ok_or(AppError::MessageNotFound)?;
    let visible = "(f.role NOT IN ('trash', 'spam') OR m.id = ?3)";
    let mut st = conn
        .prepare_cached(&format!(
            "SELECT {LISTED_COLUMNS}, mf.folder_id, m.id, lower(m.from_address) = ?4
             FROM message m
             JOIN message_folder mf ON mf.message_id = m.id
             JOIN folder f ON f.id = mf.folder_id
             LEFT JOIN message_followup fu ON fu.message_id = m.id
             WHERE m.account_id = ?1 AND (m.id = ?3 OR (?2 <> '' AND m.thread_id = ?2)) AND {visible}
               AND f.position = (SELECT min(f.position) FROM message_folder mf JOIN folder f ON f.id = mf.folder_id WHERE mf.message_id = m.id AND {visible})
             ORDER BY m.date, m.id"
        ))
        .map_err(sql_error("conversa"))?;
    let thread = thread.unwrap_or_default();
    let rows = st
        .query_map(params![account_id, thread, message_id, own.trim().to_lowercase()], |r| {
            Ok(ConversationItem { message: listed_from(r)?, folder_id: r.get(14)?, message_id: r.get(15)?, mine: r.get(16)? })
        })
        .map_err(sql_error("conversa"))?;
    let mut out: Vec<ConversationItem> = rows.collect::<Result<_, _>>().map_err(sql_error("conversa"))?;
    out.dedup_by_key(|i| i.message_id);
    Ok(out)
}

/// Os UIDs dados e os das outras mensagens das mesmas conversas na mesma pasta (US8: agir sobre
/// uma linha da lista age sobre a conversa inteira dela na pasta).
pub fn with_thread_uids(conn: &Connection, folder_id: i64, uids: &[u32]) -> Result<Vec<u32>, AppError> {
    let mut st = conn
        .prepare_cached(
            "SELECT mf2.uid FROM message_folder mf
             JOIN message m ON m.id = mf.message_id
             JOIN message m2 ON m2.account_id = m.account_id AND m2.thread_id = m.thread_id
             JOIN message_folder mf2 ON mf2.message_id = m2.id AND mf2.folder_id = mf.folder_id
             WHERE mf.folder_id = ?1 AND mf.uid = ?2 AND m.thread_id IS NOT NULL AND m.thread_id <> ''
               AND NOT EXISTS (SELECT 1 FROM pending_move pm WHERE pm.from_folder_id = mf2.folder_id AND pm.from_uid = mf2.uid)",
        )
        .map_err(sql_error("conversa"))?;
    let mut out: Vec<u32> = uids.to_vec();
    for uid in uids {
        let rows = st.query_map(params![folder_id, uid], |r| r.get::<_, u32>(0)).map_err(sql_error("conversa"))?;
        for row in rows {
            let other = row.map_err(sql_error("conversa"))?;
            if !out.contains(&other) {
                out.push(other);
            }
        }
    }
    Ok(out)
}

/// UIDs da pasta cujas mensagens ainda não têm a conversa (salvas antes da US8).
pub fn uids_without_thread(conn: &Connection, folder_id: i64, limit: u32) -> Result<Vec<u32>, AppError> {
    let mut st = conn
        .prepare_cached(
            "SELECT mf.uid FROM message_folder mf JOIN message m ON m.id = mf.message_id
             WHERE mf.folder_id = ?1 AND m.thread_id IS NULL ORDER BY mf.uid DESC LIMIT ?2",
        )
        .map_err(sql_error("conversa"))?;
    let rows = st.query_map(params![folder_id, limit], |r| r.get::<_, u32>(0)).map_err(sql_error("conversa"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("conversa"))
}

/// Grava os cabeçalhos da conversa buscados depois (mensagens salvas antes da US8).
pub fn save_threads(conn: &mut Connection, folder_id: i64, items: &[(u32, HeaderFields)]) -> Result<(), AppError> {
    let folder = get_folder(conn, folder_id)?.ok_or(AppError::InvalidId)?;
    let tx = conn.transaction().map_err(sql_error("conversa"))?;
    for (uid, fields) in items {
        let thread = thread_for(&tx, &folder.account_id, fields)?;
        tx.execute(
            "UPDATE message SET msgid = ?3, refs = ?4, thread_id = ?5
             WHERE id = (SELECT message_id FROM message_folder WHERE folder_id = ?1 AND uid = ?2) AND thread_id IS NULL",
            params![folder_id, uid, fields.message_id.as_deref().map(crate::message::normalize_id), refs_of(fields), thread],
        )
        .map_err(sql_error("conversa"))?;
    }
    tx.commit().map_err(sql_error("conversa"))
}

/// Das mensagens dadas, as que respondem a um e-mail enviado pela conta (`own`): `In-Reply-To` ou
/// `References` com o `Message-ID` de uma mensagem cujo remetente é a própria conta.
pub fn replies_to_me(conn: &Connection, account_id: &str, own: &str, ids: &[i64]) -> Result<Vec<i64>, AppError> {
    let own = own.trim().to_lowercase();
    let mut refs_of_message = conn.prepare_cached("SELECT refs FROM message WHERE id = ?1 AND account_id = ?2").map_err(sql_error("conversa"))?;
    let mut mine = conn
        .prepare_cached("SELECT 1 FROM message WHERE account_id = ?1 AND msgid = ?2 AND lower(from_address) = ?3 LIMIT 1")
        .map_err(sql_error("conversa"))?;
    let mut out = Vec::new();
    for &id in ids {
        let refs: Option<String> = refs_of_message.query_row(params![id, account_id], |r| r.get(0)).optional().map_err(sql_error("conversa"))?.flatten();
        let Some(refs) = refs else { continue };
        for token in refs.split_whitespace() {
            if mine.exists(params![account_id, token, own]).map_err(sql_error("conversa"))? {
                out.push(id);
                break;
            }
        }
    }
    Ok(out)
}

/// O que a notificação de um e-mail novo mostra (US8). Só para o módulo de notificações.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMail {
    pub folder_id: i64,
    pub uid: u32,
    pub from_name: String,
    pub from_address: String,
    pub subject: String,
    pub snippet: Option<String>,
    pub seen: bool,
}

pub fn new_mail(conn: &Connection, folder_id: i64, ids: &[i64]) -> Result<Vec<NewMail>, AppError> {
    let mut st = conn
        .prepare_cached(
            "SELECT mf.uid, m.from_name, m.from_address, m.subject, m.snippet, m.seen
             FROM message m JOIN message_folder mf ON mf.message_id = m.id WHERE m.id = ?1 AND mf.folder_id = ?2",
        )
        .map_err(sql_error("notificação"))?;
    let mut out = Vec::new();
    for &id in ids {
        if let Some(m) = st
            .query_row(params![id, folder_id], |r| {
                Ok(NewMail { folder_id, uid: r.get(0)?, from_name: r.get(1)?, from_address: r.get(2)?, subject: r.get(3)?, snippet: r.get(4)?, seen: r.get(5)? })
            })
            .optional()
            .map_err(sql_error("notificação"))?
        {
            out.push(m);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- notificações (US8, CA4)

/// Preferências de notificação e da bandeja. Valem para o app; `accounts_off` lista as contas
/// silenciadas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifySettings {
    pub new_mail: bool,
    pub replies: bool,
    pub due: bool,
    /// Hora do aviso diário dos prazos (0–23).
    pub due_hour: u32,
    pub failures: bool,
    pub reauth: bool,
    pub offline: bool,
    /// `"full"`: remetente e assunto; `"account"`: só a conta.
    pub privacy: String,
    pub quiet: bool,
    pub quiet_from: u32,
    pub quiet_to: u32,
    pub tray_on_close: bool,
    pub autostart: bool,
    pub accounts_off: Vec<String>,
}

impl Default for NotifySettings {
    fn default() -> Self {
        NotifySettings {
            new_mail: true,
            replies: true,
            due: true,
            due_hour: 9,
            failures: true,
            reauth: true,
            offline: false,
            // "Só a conta" (US10, D5): o Windows guarda o texto das notificações sem cifra.
            privacy: "account".into(),
            quiet: true,
            quiet_from: 22,
            quiet_to: 7,
            tray_on_close: true,
            autostart: false,
            accounts_off: Vec::new(),
        }
    }
}

const APP_NOTIFY: &str = "notify";
const SETTING_NOTIFY_OFF: &str = "notify_off";

pub fn notify_settings(conn: &Connection) -> Result<NotifySettings, AppError> {
    let mut settings: NotifySettings = app_setting(conn, APP_NOTIFY)?.and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default();
    let mut st = conn
        .prepare_cached("SELECT account_id FROM account_setting WHERE key = ?1 AND value = '1'")
        .map_err(sql_error("notificação"))?;
    let rows = st.query_map([SETTING_NOTIFY_OFF], |r| r.get::<_, String>(0)).map_err(sql_error("notificação"))?;
    settings.accounts_off = rows.collect::<Result<_, _>>().map_err(sql_error("notificação"))?;
    Ok(settings)
}

pub fn set_notify_settings(conn: &mut Connection, settings: &NotifySettings) -> Result<NotifySettings, AppError> {
    let valid_hour = |h: u32| h < 24;
    if !["full", "account"].contains(&settings.privacy.as_str()) || ![settings.due_hour, settings.quiet_from, settings.quiet_to].iter().all(|h| valid_hour(*h)) {
        return Err(AppError::InvalidInput);
    }
    let tx = conn.transaction().map_err(sql_error("notificação"))?;
    let stored = NotifySettings { accounts_off: Vec::new(), ..settings.clone() };
    set_app_setting(&tx, APP_NOTIFY, &serde_json::to_string(&stored).map_err(|_| AppError::InvalidInput)?)?;
    tx.execute("DELETE FROM account_setting WHERE key = ?1", [SETTING_NOTIFY_OFF]).map_err(sql_error("notificação"))?;
    for id in &settings.accounts_off {
        require_account(&tx, id)?;
        tx.execute("INSERT INTO account_setting (account_id, key, value) VALUES (?1, ?2, '1')", params![id, SETTING_NOTIFY_OFF])
            .map_err(sql_error("notificação"))?;
    }
    tx.commit().map_err(sql_error("notificação"))?;
    notify_settings(conn)
}

/// "A primeira vez que a janela fecha" (aviso da bandeja) já foi mostrada.
pub fn take_tray_hint(conn: &Connection) -> Result<bool, AppError> {
    if app_setting(conn, "tray_hint_shown")?.is_some() {
        return Ok(false);
    }
    set_app_setting(conn, "tray_hint_shown", "1")?;
    Ok(true)
}

/// Não lidas da Caixa de entrada de todas as contas ativas (tooltip da bandeja e ponto na barra).
pub fn inbox_unread_total(conn: &Connection) -> Result<u32, AppError> {
    conn.query_row(
        "SELECT coalesce(sum(f.unread), 0) FROM folder f JOIN account a ON a.id = f.account_id WHERE f.role = 'inbox' AND a.status = 'active'",
        [],
        |r| r.get(0),
    )
    .map_err(sql_error("contas"))
}

// ---------------------------------------------------------------- sino (US8, CA7)

/// Por quanto tempo o sino guarda as notificações (D12).
pub const NOTICE_DAYS: i64 = 7;
const NOTICE_LIMIT: u32 = 100;

/// Uma notificação a guardar no histórico do sino.
#[derive(Debug, Clone)]
pub struct NewNotice {
    pub account_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub line1: String,
    pub line2: String,
    /// Remetente, para o avatar. Só com a privacidade "Remetente e assunto".
    pub sender: Option<String>,
    pub target: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub id: i64,
    pub account_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub line1: String,
    pub line2: String,
    pub sender: Option<String>,
    pub target: Option<serde_json::Value>,
    pub created_at: i64,
    pub seen: bool,
}

/// Guarda as notificações e apaga as mais antigas que o prazo do sino.
pub fn add_notices(conn: &mut Connection, notices: &[NewNotice], at: i64) -> Result<(), AppError> {
    let tx = conn.transaction().map_err(sql_error("sino"))?;
    {
        let mut st = tx
            .prepare_cached("INSERT INTO notice (account_id, kind, title, line1, line2, sender, target, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")
            .map_err(sql_error("sino"))?;
        for n in notices {
            let target = n.target.as_ref().map(|t| t.to_string());
            st.execute(params![n.account_id, n.kind, n.title, n.line1, n.line2, n.sender, target, at]).map_err(sql_error("sino"))?;
        }
    }
    tx.execute("DELETE FROM notice WHERE created_at < ?1", [at - NOTICE_DAYS * 86_400]).map_err(sql_error("sino"))?;
    tx.commit().map_err(sql_error("sino"))
}

/// As notificações dos últimos 7 dias, das mais novas para as mais antigas.
pub fn list_notices(conn: &Connection, at: i64) -> Result<Vec<Notice>, AppError> {
    let mut st = conn
        .prepare_cached(
            "SELECT id, account_id, kind, title, line1, line2, sender, target, created_at, seen FROM notice
             WHERE created_at >= ?1 ORDER BY created_at DESC, id DESC LIMIT ?2",
        )
        .map_err(sql_error("sino"))?;
    let rows = st
        .query_map(params![at - NOTICE_DAYS * 86_400, NOTICE_LIMIT], |r| {
            Ok(Notice {
                id: r.get(0)?,
                account_id: r.get(1)?,
                kind: r.get(2)?,
                title: r.get(3)?,
                line1: r.get(4)?,
                line2: r.get(5)?,
                sender: r.get(6)?,
                target: r.get::<_, Option<String>>(7)?.and_then(|t| serde_json::from_str(&t).ok()),
                created_at: r.get(8)?,
                seen: r.get(9)?,
            })
        })
        .map_err(sql_error("sino"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("sino"))
}

/// Abrir o painel ou "Marcar todas como vistas".
pub fn mark_notices_seen(conn: &Connection) -> Result<(), AppError> {
    conn.execute("UPDATE notice SET seen = 1 WHERE seen = 0", []).map_err(sql_error("sino"))?;
    Ok(())
}

// ---------------------------------------------------------------- privacidade (US10)

/// "Bloquear depois de" (CA3, D2: 15 minutos) e "Bloquear junto com o Windows" (D2: ligado).
pub const LOCK_CHOICES: &[&str] = &["5", "15", "30", "60", "never"];
const LOCK_DEFAULT: &str = "15";
const APP_LOCK_MINUTES: &str = "lock_minutes";
const APP_LOCK_WITH_WINDOWS: &str = "lock_with_windows";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LockSettings {
    pub minutes: String,
    pub with_windows: bool,
}

pub fn lock_settings(conn: &Connection) -> Result<LockSettings, AppError> {
    let minutes = app_setting(conn, APP_LOCK_MINUTES)?.filter(|v| LOCK_CHOICES.contains(&v.as_str())).unwrap_or_else(|| LOCK_DEFAULT.to_owned());
    let with_windows = app_setting(conn, APP_LOCK_WITH_WINDOWS)?.as_deref() != Some("0");
    Ok(LockSettings { minutes, with_windows })
}

pub fn set_lock_settings(conn: &Connection, settings: &LockSettings) -> Result<LockSettings, AppError> {
    if !LOCK_CHOICES.contains(&settings.minutes.as_str()) {
        return Err(AppError::InvalidInput);
    }
    set_app_setting(conn, APP_LOCK_MINUTES, &settings.minutes)?;
    set_app_setting(conn, APP_LOCK_WITH_WINDOWS, if settings.with_windows { "1" } else { "0" })?;
    lock_settings(conn)
}

/// Espaço dos e-mails guardados (conteúdo baixado) por conta (CA7).
pub fn stored_bytes_by_account(conn: &Connection) -> Result<Vec<(String, u64)>, AppError> {
    let mut stmt = conn
        .prepare("SELECT m.account_id, SUM(length(b.raw)) FROM message_body b JOIN message m ON m.id = b.message_id GROUP BY m.account_id")
        .map_err(sql_error("espaço por conta"))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<i64>>(1)?.unwrap_or(0).max(0) as u64)))
        .map_err(sql_error("espaço por conta"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("espaço por conta"))
}

/// Mensagens fora do período escolhido: anteriores a `before`, ou todas com `None` ("Só ao abrir").
const OUTSIDE: &str = "SELECT id FROM message WHERE body_state = 'cached' AND (?1 IS NULL OR date < ?1)";

/// Bytes do conteúdo guardado fora do período (CA7).
pub fn bytes_outside(conn: &Connection, before: Option<&str>) -> Result<u64, AppError> {
    let bytes: Option<i64> = conn
        .query_row(&format!("SELECT SUM(length(raw)) FROM message_body WHERE message_id IN ({OUTSIDE})"), [before], |r| r.get(0))
        .map_err(sql_error("fora do período"))?;
    Ok(bytes.unwrap_or(0).max(0) as u64)
}

/// Apaga o conteúdo guardado fora do período (CA7, D7). Os cabeçalhos ficam: o e-mail continua na
/// lista e baixa de novo ao abrir. Devolve quantas mensagens perderam o conteúdo.
pub fn purge_outside(conn: &mut Connection, before: Option<&str>) -> Result<usize, AppError> {
    let tx = conn.transaction().map_err(sql_error("apagar fora do período"))?;
    let ids: Vec<i64> = {
        let mut stmt = tx.prepare(OUTSIDE).map_err(sql_error("apagar fora do período"))?;
        let rows = stmt.query_map([before], |r| r.get(0)).map_err(sql_error("apagar fora do período"))?;
        rows.collect::<Result<_, _>>().map_err(sql_error("apagar fora do período"))?
    };
    for id in &ids {
        tx.execute("DELETE FROM message_body WHERE message_id = ?1", [id]).map_err(sql_error("apagar fora do período"))?;
        tx.execute("UPDATE message SET body_state = 'none' WHERE id = ?1", [id]).map_err(sql_error("apagar fora do período"))?;
    }
    tx.commit().map_err(sql_error("apagar fora do período"))?;
    Ok(ids.len())
}

/// Apaga uma conta de que o usuário saiu, com tudo que pende dela (mensagens, pastas, rascunhos,
/// avisos), por `ON DELETE CASCADE` (CA7). Conta ativa: `InvalidInput` (para isso, sair antes).
pub fn delete_signed_out_account(conn: &Connection, account_id: &str) -> Result<(), AppError> {
    if require_account(conn, account_id)?.status != AccountStatus::SignedOut {
        return Err(AppError::InvalidInput);
    }
    conn.execute("DELETE FROM account WHERE id = ?1", [account_id]).map_err(sql_error("apagar conta"))?;
    Ok(())
}

/// Depois de apagar dados: o arquivo encolhe e o WAL é zerado.
pub fn compact(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch("VACUUM;").map_err(sql_error("compactar"))?;
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(())).map_err(sql_error("compactar"))?;
    Ok(())
}

// ---------------------------------------------------------------- download em segundo plano (US9)

/// "Baixar e-mails para ler sem conexão" (CA4): últimos 30 ou 90 dias, tudo ou só ao abrir (D1: 30).
pub const OFFLINE_CHOICES: &[&str] = &["30", "90", "all", "none"];
pub const OFFLINE_DEFAULT: &str = "30";
const APP_OFFLINE: &str = "offline_days";

pub fn offline_days(conn: &Connection) -> Result<String, AppError> {
    Ok(app_setting(conn, APP_OFFLINE)?.filter(|v| OFFLINE_CHOICES.contains(&v.as_str())).unwrap_or_else(|| OFFLINE_DEFAULT.to_owned()))
}

pub fn set_offline_days(conn: &Connection, choice: &str) -> Result<String, AppError> {
    if !OFFLINE_CHOICES.contains(&choice) {
        return Err(AppError::InvalidInput);
    }
    set_app_setting(conn, APP_OFFLINE, choice)?;
    Ok(choice.to_owned())
}

/// Desde que data baixar ("AAAA-MM-DD"): `Some(None)` é tudo; `None` é não baixar nada antes de abrir.
pub fn offline_since(choice: &str, today: chrono::NaiveDate) -> Option<Option<String>> {
    let days = match choice {
        "none" => return None,
        "all" => return Some(None),
        n => n.parse::<i64>().ok()?,
    };
    Some(Some((today - chrono::Duration::days(days)).format("%Y-%m-%d").to_string()))
}

/// Uma mensagem cujo corpo falta baixar, com o lugar de onde buscar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyTarget {
    pub message_id: i64,
    pub folder_id: i64,
    pub uid: u32,
}

/// O próximo lote do download em segundo plano (CA2): mensagens sem corpo salvo, até `max_size`
/// bytes e desde `since`, todas da mesma pasta. Ordem: Caixa de entrada, Enviados, as outras pastas
/// na ordem da coluna e "Todos" por último (repete as outras); dentro da pasta, das mais novas para as
/// mais antigas. Lixeira, Spam e mensagens sendo movidas ficam de fora; `skip` são as já tentadas.
pub fn bodies_to_download(
    conn: &Connection,
    account_id: &str,
    since: Option<&str>,
    max_size: u32,
    limit: usize,
    skip: &HashSet<i64>,
) -> Result<Vec<BodyTarget>, AppError> {
    let mut st = conn
        .prepare_cached(
            "SELECT m.id, mf.folder_id, mf.uid FROM message m
             JOIN message_folder mf ON mf.message_id = m.id
             JOIN folder f ON f.id = mf.folder_id
             WHERE m.account_id = ?1 AND m.body_state = 'none' AND m.size <= ?2 AND (?3 IS NULL OR m.date >= ?3)
               AND f.role NOT IN ('trash', 'spam')
               AND NOT EXISTS (SELECT 1 FROM pending_move pm WHERE pm.from_folder_id = mf.folder_id AND pm.from_uid = mf.uid)
             ORDER BY CASE f.role WHEN 'inbox' THEN 0 WHEN 'sent' THEN 1 WHEN 'all' THEN 3 ELSE 2 END, f.position, m.date DESC, mf.uid DESC
             LIMIT ?4",
        )
        .map_err(sql_error("download"))?;
    let window = (limit + skip.len()).saturating_mul(4).max(100) as i64;
    let rows = st
        .query_map(params![account_id, max_size, since, window], |r| Ok(BodyTarget { message_id: r.get(0)?, folder_id: r.get(1)?, uid: r.get(2)? }))
        .map_err(sql_error("download"))?;
    let mut out: Vec<BodyTarget> = Vec::new();
    for row in rows {
        let t = row.map_err(sql_error("download"))?;
        if skip.contains(&t.message_id) || out.iter().any(|o| o.message_id == t.message_id) {
            continue;
        }
        if out.first().is_some_and(|first| first.folder_id != t.folder_id) {
            continue;
        }
        out.push(t);
        if out.len() >= limit {
            break;
        }
    }
    Ok(out)
}

/// Das mensagens pedidas (pasta, UID), as que ainda não têm corpo e cabem em `max_size` (CA3).
pub fn bodies_for_keys(conn: &Connection, account_id: &str, keys: &[(i64, u32)], max_size: u32) -> Result<Vec<BodyTarget>, AppError> {
    let mut st = conn
        .prepare_cached(
            "SELECT m.id FROM message_folder mf JOIN message m ON m.id = mf.message_id
             WHERE mf.folder_id = ?1 AND mf.uid = ?2 AND m.account_id = ?3 AND m.body_state = 'none' AND m.size <= ?4",
        )
        .map_err(sql_error("download"))?;
    let mut out = Vec::new();
    for &(folder_id, uid) in keys {
        let id: Option<i64> = st.query_row(params![folder_id, uid, account_id, max_size], |r| r.get(0)).optional().map_err(sql_error("download"))?;
        if let Some(message_id) = id.filter(|id| !out.iter().any(|t: &BodyTarget| t.message_id == *id)) {
            out.push(BodyTarget { message_id, folder_id, uid });
        }
    }
    Ok(out)
}

/// Mensagens da pasta salvas no banco (a primeira sincronização completa até `INITIAL_WINDOW`, CA1).
pub fn stored_in_folder(conn: &Connection, folder_id: i64) -> Result<u32, AppError> {
    conn.query_row("SELECT count(*) FROM message_folder WHERE folder_id = ?1", [folder_id], |r| r.get(0)).map_err(sql_error("pasta"))
}

// ---------------------------------------------------------------- prévia da lista (US7, CA7)

/// UIDs da pasta, dos mais novos para os mais antigos, cuja prévia ainda não foi buscada.
pub fn uids_without_preview(conn: &Connection, folder_id: i64, limit: u32) -> Result<Vec<u32>, AppError> {
    let mut st = conn
        .prepare_cached(
            "SELECT mf.uid FROM message_folder mf JOIN message m ON m.id = mf.message_id
             WHERE mf.folder_id = ?1 AND m.snippet IS NULL ORDER BY mf.uid DESC LIMIT ?2",
        )
        .map_err(sql_error("prévia"))?;
    let rows = st.query_map(params![folder_id, limit], |r| r.get::<_, u32>(0)).map_err(sql_error("prévia"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("prévia"))
}

pub fn save_previews(conn: &mut Connection, folder_id: i64, previews: &[(u32, crate::preview::Preview)]) -> Result<(), AppError> {
    let tx = conn.transaction().map_err(sql_error("prévia"))?;
    {
        let mut st = tx
            .prepare_cached(
                "UPDATE message SET snippet = ?3, has_files = ?4, has_images = ?5
                 WHERE id = (SELECT message_id FROM message_folder WHERE folder_id = ?1 AND uid = ?2)",
            )
            .map_err(sql_error("prévia"))?;
        for (uid, p) in previews {
            st.execute(params![folder_id, uid, p.snippet, p.has_files, p.has_images]).map_err(sql_error("prévia"))?;
        }
    }
    tx.commit().map_err(sql_error("prévia"))
}

pub fn mark_too_large(conn: &Connection, message_id: i64) -> Result<(), AppError> {
    conn.execute("UPDATE message SET body_state = 'too_large' WHERE id = ?1", [message_id]).map_err(sql_error("corpo"))?;
    Ok(())
}

/// Grava as flags confirmadas pelo servidor e ajusta o contador de não lidas de toda pasta em que
/// a mensagem está, sem esperar a próxima sincronização.
pub fn set_local_flags(conn: &mut Connection, message_id: i64, flags: MessageFlags) -> Result<(), AppError> {
    let tx = conn.transaction().map_err(sql_error("flags"))?;
    let previous: Option<bool> = tx
        .query_row("SELECT seen FROM message WHERE id = ?1", [message_id], |r| r.get(0))
        .optional()
        .map_err(sql_error("flags"))?;
    let Some(was_seen) = previous else { return Ok(()) };
    tx.execute("UPDATE message SET seen = ?2, flagged = ?3 WHERE id = ?1", params![message_id, flags.seen, flags.flagged])
        .map_err(sql_error("flags"))?;
    if was_seen != flags.seen {
        let delta: i64 = if flags.seen { -1 } else { 1 };
        tx.execute(
            "UPDATE folder SET unread = max(0, unread + ?2)
             WHERE unread IS NOT NULL AND id IN (SELECT folder_id FROM message_folder WHERE message_id = ?1)",
            params![message_id, delta],
        )
        .map_err(sql_error("flags"))?;
    }
    tx.commit().map_err(sql_error("flags"))
}

// ---------------------------------------------------------------- busca local (US4, CA1)

pub const SEARCH_LIMIT: u32 = 200;

/// Escapa `%`, `_` e `\` para um `LIKE ... ESCAPE '\'`.
pub fn like_pattern(query: &str) -> String {
    let mut out = String::with_capacity(query.len() + 2);
    out.push('%');
    for c in query.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub folder_id: i64,
    pub folder_name: String,
    pub message: ListedMessage,
}

/// Busca nas mensagens salvas da conta (assunto, remetente, destinatários). Cada mensagem aparece
/// uma vez, na primeira pasta da ordem da sidebar em que está. Devolve até `SEARCH_LIMIT + 1`.
pub fn search(conn: &Connection, account_id: &str, query: &str) -> Result<Vec<SearchHit>, AppError> {
    let pattern = like_pattern(query.trim());
    let mut st = conn
        .prepare_cached(&format!(
            "SELECT {LISTED_COLUMNS}, f.id, f.display_name
             FROM message m
             JOIN message_folder mf ON mf.message_id = m.id
             JOIN folder f ON f.id = mf.folder_id
             LEFT JOIN message_followup fu ON fu.message_id = m.id
             WHERE m.account_id = ?1
               AND (m.subject LIKE ?2 ESCAPE '\\' OR m.from_name LIKE ?2 ESCAPE '\\' OR m.from_address LIKE ?2 ESCAPE '\\'
                    OR m.to_json LIKE ?2 ESCAPE '\\' OR m.cc_json LIKE ?2 ESCAPE '\\')
               AND f.position = (SELECT min(f2.position) FROM message_folder mf2 JOIN folder f2 ON f2.id = mf2.folder_id WHERE mf2.message_id = m.id)
             ORDER BY m.date DESC LIMIT ?3"
        ))
        .map_err(sql_error("busca"))?;
    let rows = st
        .query_map(params![account_id, pattern, SEARCH_LIMIT + 1], |r| {
            Ok(SearchHit { message: listed_from(r)?, folder_id: r.get(14)?, folder_name: r.get(15)? })
        })
        .map_err(sql_error("busca"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("busca"))
}

// ---------------------------------------------------------------- sinalizadas (US6, CA6)

/// Prazo das sinalizadas na coluna de pastas. "Esta semana" vai de amanhã até o fim da semana
/// calculado no frontend (a data local do usuário), como na US4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagBucket {
    Overdue,
    Today,
    ThisWeek,
    Later,
    NoDate,
}

/// Datas de referência (AAAA-MM-DD), validadas por quem chama.
#[derive(Debug, Clone)]
pub struct FlagDates {
    pub today: String,
    pub week_end: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlaggedCounts {
    pub overdue: u32,
    pub today: u32,
    pub this_week: u32,
    pub later: u32,
    pub no_date: u32,
}

/// Sinalizada e não concluída, com a estrela ainda ligada (a sincronização apaga o prazo quando a
/// estrela some em outro dispositivo).
const FLAGGED_WHERE: &str = "fu.completed_at IS NULL AND m.flagged = 1 AND m.account_id = ?1";

const BUCKET_CASE: &str = "CASE
        WHEN fu.due_date IS NULL THEN 'no_date'
        WHEN fu.due_date < ?2 THEN 'overdue'
        WHEN fu.due_date = ?2 THEN 'today'
        WHEN fu.due_date <= ?3 THEN 'this_week'
        ELSE 'later' END";

pub fn flagged_counts(conn: &Connection, account_id: &str, dates: &FlagDates) -> Result<FlaggedCounts, AppError> {
    let mut st = conn
        .prepare_cached(&format!(
            "SELECT {BUCKET_CASE} AS bucket, count(*)
             FROM message m JOIN message_followup fu ON fu.message_id = m.id
             WHERE {FLAGGED_WHERE} AND EXISTS (SELECT 1 FROM message_folder mf WHERE mf.message_id = m.id)
             GROUP BY bucket"
        ))
        .map_err(sql_error("sinalizadas"))?;
    let rows = st
        .query_map(params![account_id, dates.today, dates.week_end], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?)))
        .map_err(sql_error("sinalizadas"))?;
    let mut counts = FlaggedCounts::default();
    for row in rows {
        let (bucket, n) = row.map_err(sql_error("sinalizadas"))?;
        match bucket.as_str() {
            "overdue" => counts.overdue = n,
            "today" => counts.today = n,
            "this_week" => counts.this_week = n,
            "later" => counts.later = n,
            _ => counts.no_date = n,
        }
    }
    Ok(counts)
}

fn bucket_name(bucket: FlagBucket) -> &'static str {
    match bucket {
        FlagBucket::Overdue => "overdue",
        FlagBucket::Today => "today",
        FlagBucket::ThisWeek => "this_week",
        FlagBucket::Later => "later",
        FlagBucket::NoDate => "no_date",
    }
}

/// Mensagens de um prazo, como na busca: uma vez cada, na primeira pasta da sidebar. Atrasadas e
/// com prazo vêm pelo prazo mais próximo; sem data, pelas mais recentes. Até `SEARCH_LIMIT + 1`.
pub fn flagged(conn: &Connection, account_id: &str, bucket: FlagBucket, dates: &FlagDates) -> Result<Vec<SearchHit>, AppError> {
    let mut st = conn
        .prepare_cached(&format!(
            "SELECT {LISTED_COLUMNS}, f.id, f.display_name
             FROM message m
             JOIN message_folder mf ON mf.message_id = m.id
             JOIN folder f ON f.id = mf.folder_id
             JOIN message_followup fu ON fu.message_id = m.id
             WHERE {FLAGGED_WHERE} AND {BUCKET_CASE} = ?4
               AND f.position = (SELECT min(f2.position) FROM message_folder mf2 JOIN folder f2 ON f2.id = mf2.folder_id WHERE mf2.message_id = m.id)
             ORDER BY fu.due_date IS NULL, fu.due_date, m.date DESC LIMIT ?5"
        ))
        .map_err(sql_error("sinalizadas"))?;
    let rows = st
        .query_map(params![account_id, dates.today, dates.week_end, bucket_name(bucket), SEARCH_LIMIT + 1], |r| {
            Ok(SearchHit { message: listed_from(r)?, folder_id: r.get(14)?, folder_name: r.get(15)? })
        })
        .map_err(sql_error("sinalizadas"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("sinalizadas"))
}

// ---------------------------------------------------------------- aparência (US6, CA3–CA4)

const APP_THEME: &str = "theme";
const APP_CANVAS: &str = "canvas";
pub const THEMES: &[&str] = &["light", "dark", "system"];
/// Cores do fundo; "" é o padrão (`--color-canvas`).
pub const CANVASES: &[&str] = &["", "blue", "teal", "green", "purple", "magenta", "orange", "sand"];

pub const DENSITIES: &[&str] = &["comfortable", "standard", "compact"];
const APP_DENSITY: &str = "density";
const APP_SHOW_FOLDERS: &str = "show_folders";
const APP_SHOW_READER: &str = "show_reader";

/// Aparência e preferências de lista (US6 e US7): valem para todas as contas deste computador.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    pub theme: String,
    pub canvas: String,
    pub density: String,
    pub show_folders: bool,
    pub show_reader: bool,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceChange {
    pub theme: Option<String>,
    pub canvas: Option<String>,
    pub density: Option<String>,
    pub show_folders: Option<bool>,
    pub show_reader: Option<bool>,
}

fn app_setting(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    conn.query_row("SELECT value FROM app_setting WHERE key = ?1", [key], |r| r.get(0))
        .optional()
        .map_err(sql_error("ajuste do app"))
}

fn set_app_setting(conn: &Connection, key: &str, value: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO app_setting (key, value) VALUES (?1, ?2) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(sql_error("ajuste do app"))?;
    Ok(())
}

pub fn appearance(conn: &Connection) -> Result<Appearance, AppError> {
    let theme = app_setting(conn, APP_THEME)?.filter(|t| THEMES.contains(&t.as_str())).unwrap_or_else(|| "system".into());
    let canvas = app_setting(conn, APP_CANVAS)?.filter(|c| CANVASES.contains(&c.as_str())).unwrap_or_default();
    let density = app_setting(conn, APP_DENSITY)?.filter(|d| DENSITIES.contains(&d.as_str())).unwrap_or_else(|| "standard".into());
    let shown = |key| Ok::<_, AppError>(app_setting(conn, key)?.as_deref() != Some("0"));
    Ok(Appearance { theme, canvas, density, show_folders: shown(APP_SHOW_FOLDERS)?, show_reader: shown(APP_SHOW_READER)? })
}

pub fn set_appearance(conn: &Connection, change: &AppearanceChange) -> Result<Appearance, AppError> {
    let invalid = change.theme.as_deref().is_some_and(|t| !THEMES.contains(&t))
        || change.canvas.as_deref().is_some_and(|c| !CANVASES.contains(&c))
        || change.density.as_deref().is_some_and(|d| !DENSITIES.contains(&d));
    if invalid {
        return Err(AppError::InvalidInput);
    }
    if let Some(theme) = &change.theme {
        set_app_setting(conn, APP_THEME, theme)?;
    }
    if let Some(canvas) = &change.canvas {
        set_app_setting(conn, APP_CANVAS, canvas)?;
    }
    if let Some(density) = &change.density {
        set_app_setting(conn, APP_DENSITY, density)?;
    }
    if let Some(show) = change.show_folders {
        set_app_setting(conn, APP_SHOW_FOLDERS, if show { "1" } else { "0" })?;
    }
    if let Some(show) = change.show_reader {
        set_app_setting(conn, APP_SHOW_READER, if show { "1" } else { "0" })?;
    }
    appearance(conn)
}

// ---------------------------------------------------------------- menu da conta (US7, CA6)

/// Informações da seção 1 do menu da conta. Só números; nada cita o provedor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountOverview {
    /// Última sincronização bem-sucedida (segundos Unix).
    pub last_sync_at: Option<i64>,
    /// Mensagens não lidas salvas da conta, cada uma uma vez (fora Lixeira, Spam e "Todos").
    pub unread: u32,
    /// Mensagens salvas da conta neste computador.
    pub saved: u32,
}

pub fn account_overview(conn: &Connection, account_id: &str) -> Result<AccountOverview, AppError> {
    conn.query_row(
        "SELECT a.last_sync_at,
            (SELECT count(DISTINCT m.id) FROM message m JOIN message_folder mf ON mf.message_id = m.id JOIN folder f ON f.id = mf.folder_id
              WHERE m.account_id = a.id AND m.seen = 0 AND f.role NOT IN ('trash', 'spam', 'all')),
            (SELECT count(*) FROM message m WHERE m.account_id = a.id)
         FROM account a WHERE a.id = ?1",
        [account_id],
        |r| Ok(AccountOverview { last_sync_at: r.get(0)?, unread: r.get(1)?, saved: r.get(2)? }),
    )
    .optional()
    .map_err(sql_error("conta"))?
    .ok_or(AppError::AccountNotFound)
}

/// Nome de quem envia (claim `name` do provedor), para o cabeçalho `From` das mensagens enviadas.
pub fn set_sender_name(conn: &Connection, account_id: &str, name: Option<&str>) -> Result<(), AppError> {
    conn.execute("UPDATE account SET sender_name = ?2 WHERE id = ?1", params![account_id, name]).map_err(sql_error("conta"))?;
    Ok(())
}

pub fn sender_name(conn: &Connection, account_id: &str) -> Result<Option<String>, AppError> {
    conn.query_row("SELECT sender_name FROM account WHERE id = ?1", [account_id], |r| r.get(0))
        .optional()
        .map(Option::flatten)
        .map_err(sql_error("conta"))
}

/// Servidor de envio da conta.
pub fn smtp_server(conn: &Connection, account_id: &str) -> Result<(String, u16, String), AppError> {
    conn.query_row("SELECT smtp_host, smtp_port, smtp_security FROM provider_config WHERE account_id = ?1", [account_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .optional()
        .map_err(sql_error("servidor da conta"))?
        .ok_or(AppError::AccountNotFound)
}

// ---------------------------------------------------------------- mover, excluir, arquivar (CA4, CA5)

/// Tira as mensagens da pasta no banco depois que o servidor confirmou a operação. Com `in_transit`,
/// a mensagem fica protegida da limpeza até a pasta de destino ser sincronizada.
pub fn unlink(conn: &mut Connection, folder_id: i64, uids: &[u32], in_transit: bool) -> Result<usize, AppError> {
    let folder = get_folder(conn, folder_id)?.ok_or(AppError::InvalidId)?;
    let tx = conn.transaction().map_err(sql_error("mover"))?;
    let mut removed = 0;
    let mut unseen = 0;
    for uid in uids {
        let found: Option<(i64, bool)> = tx
            .query_row(
                "SELECT m.id, m.seen FROM message_folder mf JOIN message m ON m.id = mf.message_id WHERE mf.folder_id = ?1 AND mf.uid = ?2",
                params![folder_id, uid],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(sql_error("mover"))?;
        let Some((message_id, seen)) = found else { continue };
        if in_transit {
            tx.execute("UPDATE message SET in_transit_since = ?2 WHERE id = ?1", params![message_id, now()]).map_err(sql_error("mover"))?;
        }
        tx.execute("DELETE FROM message_folder WHERE folder_id = ?1 AND uid = ?2", params![folder_id, uid]).map_err(sql_error("mover"))?;
        removed += 1;
        if !seen {
            unseen += 1;
        }
    }
    tx.execute(
        "UPDATE folder SET total = max(0, coalesce(total, 0) - ?2), unread = CASE WHEN unread IS NULL THEN NULL ELSE max(0, unread - ?3) END WHERE id = ?1",
        params![folder_id, removed as i64, unseen],
    )
    .map_err(sql_error("mover"))?;
    delete_orphans(&tx, &folder.account_id)?;
    tx.commit().map_err(sql_error("mover"))?;
    Ok(removed)
}

// ---------------------------------------------------------------- mover pendente (US7, D1)

/// Mensagem a caminho de outra pasta. `waiting`: dentro do prazo do "Desfazer", nada foi ao
/// servidor; `running`: o MOVE está em andamento; `moved`: o servidor confirmou e a mensagem
/// espera a sincronização do destino para ganhar o UID de lá.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingMove {
    pub id: i64,
    pub batch: String,
    pub account_id: String,
    pub message_id: i64,
    pub from_folder_id: i64,
    pub from_uid: u32,
    pub to_folder_id: i64,
}

const PENDING_COLUMNS: &str = "id, batch, account_id, message_id, from_folder_id, from_uid, to_folder_id";

fn pending_from(r: &Row<'_>) -> rusqlite::Result<PendingMove> {
    Ok(PendingMove {
        id: r.get(0)?,
        batch: r.get(1)?,
        account_id: r.get(2)?,
        message_id: r.get(3)?,
        from_folder_id: r.get(4)?,
        from_uid: r.get(5)?,
        to_folder_id: r.get(6)?,
    })
}

/// A mensagem está ligada a esta pasta no banco (ex.: "Todos os e-mails" do Gmail já tem quase tudo).
fn linked_to(conn: &Connection, message_id: i64, folder_id: i64) -> Result<bool, AppError> {
    conn.query_row("SELECT 1 FROM message_folder WHERE message_id = ?1 AND folder_id = ?2", params![message_id, folder_id], |_| Ok(()))
        .optional()
        .map(|r| r.is_some())
        .map_err(sql_error("mover"))
}

/// Ajusta total e não lidas da pasta (`delta` +1 entra, -1 sai) sem esperar o servidor.
fn shift_counts(conn: &Connection, folder_id: i64, delta: i64, unseen: bool) -> Result<(), AppError> {
    conn.execute(
        "UPDATE folder SET total = max(0, coalesce(total, 0) + ?2),
            unread = CASE WHEN unread IS NULL OR ?3 = 0 THEN unread ELSE max(0, unread + ?2) END
         WHERE id = ?1",
        params![folder_id, delta, unseen],
    )
    .map_err(sql_error("mover"))?;
    Ok(())
}

fn move_counts(conn: &Connection, message_id: i64, from: i64, to: i64, undo: bool) -> Result<(), AppError> {
    let seen: bool = conn.query_row("SELECT seen FROM message WHERE id = ?1", [message_id], |r| r.get(0)).map_err(sql_error("mover"))?;
    let sign = if undo { -1 } else { 1 };
    shift_counts(conn, from, -sign, !seen)?;
    if !linked_to(conn, message_id, to)? {
        shift_counts(conn, to, sign, !seen)?;
    }
    Ok(())
}

/// Registra o movimento das mensagens `(pasta, uid)` para `to_folder_id`. Some da origem e aparece
/// no destino na hora (listas e contadores). Uma mensagem que já estava indo para outra pasta muda
/// de destino (ou volta, se o novo destino for a origem). Devolve as que entraram no lote; as que já
/// estão no servidor a caminho ficam de fora.
pub fn create_pending_moves(conn: &mut Connection, account_id: &str, batch: &str, items: &[(i64, u32)], to_folder_id: i64, due_at: i64) -> Result<Vec<(i64, u32)>, AppError> {
    let tx = conn.transaction().map_err(sql_error("mover"))?;
    let mut accepted = Vec::new();
    for &(folder_id, uid) in items {
        let existing: Option<(i64, i64, String)> = tx
            .query_row(
                "SELECT id, to_folder_id, state FROM pending_move WHERE from_folder_id = ?1 AND from_uid = ?2",
                params![folder_id, uid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(sql_error("mover"))?;
        let message_id: Option<i64> = tx
            .query_row("SELECT message_id FROM message_folder WHERE folder_id = ?1 AND uid = ?2", params![folder_id, uid], |r| r.get(0))
            .optional()
            .map_err(sql_error("mover"))?;
        let Some(message_id) = message_id else { continue };
        match existing {
            Some((_, _, state)) if state != "waiting" => continue,
            Some((id, old_to, _)) => {
                move_counts(&tx, message_id, folder_id, old_to, true)?;
                if to_folder_id == folder_id {
                    tx.execute("DELETE FROM pending_move WHERE id = ?1", [id]).map_err(sql_error("mover"))?;
                } else {
                    move_counts(&tx, message_id, folder_id, to_folder_id, false)?;
                    tx.execute("UPDATE pending_move SET to_folder_id = ?2, batch = ?3, due_at = ?4 WHERE id = ?1", params![id, to_folder_id, batch, due_at])
                        .map_err(sql_error("mover"))?;
                }
            }
            None if to_folder_id == folder_id => continue,
            None => {
                tx.execute(
                    "INSERT INTO pending_move (batch, account_id, message_id, from_folder_id, from_uid, to_folder_id, state, due_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'waiting', ?7)",
                    params![batch, account_id, message_id, folder_id, uid, to_folder_id, due_at],
                )
                .map_err(sql_error("mover"))?;
                move_counts(&tx, message_id, folder_id, to_folder_id, false)?;
            }
        }
        accepted.push((folder_id, uid));
    }
    tx.commit().map_err(sql_error("mover"))?;
    Ok(accepted)
}

/// "Desfazer": tira do lote o que ainda não foi ao servidor. Devolve `(pasta, uid)` desfeitos.
pub fn cancel_pending(conn: &mut Connection, batch: &str) -> Result<Vec<(i64, u32)>, AppError> {
    let tx = conn.transaction().map_err(sql_error("mover"))?;
    let rows: Vec<PendingMove> = {
        let mut st = tx
            .prepare_cached(&format!("SELECT {PENDING_COLUMNS} FROM pending_move WHERE batch = ?1 AND state = 'waiting'"))
            .map_err(sql_error("mover"))?;
        let rows = st.query_map([batch], pending_from).map_err(sql_error("mover"))?;
        rows.collect::<Result<_, _>>().map_err(sql_error("mover"))?
    };
    for m in &rows {
        move_counts(&tx, m.message_id, m.from_folder_id, m.to_folder_id, true)?;
        tx.execute("DELETE FROM pending_move WHERE id = ?1", [m.id]).map_err(sql_error("mover"))?;
    }
    tx.commit().map_err(sql_error("mover"))?;
    Ok(rows.into_iter().map(|m| (m.from_folder_id, m.from_uid)).collect())
}

/// Pega os movimentos vencidos (`due_at <= now`) e os marca como em andamento.
pub fn take_due_moves(conn: &mut Connection, now: i64) -> Result<Vec<PendingMove>, AppError> {
    let tx = conn.transaction().map_err(sql_error("mover"))?;
    let rows: Vec<PendingMove> = {
        let mut st = tx
            .prepare_cached(&format!("SELECT {PENDING_COLUMNS} FROM pending_move WHERE state = 'waiting' AND due_at <= ?1 ORDER BY id"))
            .map_err(sql_error("mover"))?;
        let rows = st.query_map([now], pending_from).map_err(sql_error("mover"))?;
        rows.collect::<Result<_, _>>().map_err(sql_error("mover"))?
    };
    for m in &rows {
        tx.execute("UPDATE pending_move SET state = 'running' WHERE id = ?1", [m.id]).map_err(sql_error("mover"))?;
    }
    tx.commit().map_err(sql_error("mover"))?;
    Ok(rows)
}

/// Resultado do MOVE no servidor. Com sucesso, a ligação com a origem sai (a mensagem fica
/// protegida da limpeza) e o registro espera a sincronização do destino. Com falha, a mensagem
/// volta para a origem e os contadores também.
pub fn finish_moves(conn: &mut Connection, ids: &[i64], ok: bool) -> Result<(), AppError> {
    let tx = conn.transaction().map_err(sql_error("mover"))?;
    for &id in ids {
        let Some(m) = tx
            .query_row(&format!("SELECT {PENDING_COLUMNS} FROM pending_move WHERE id = ?1"), [id], pending_from)
            .optional()
            .map_err(sql_error("mover"))?
        else {
            continue;
        };
        if ok {
            tx.execute("DELETE FROM message_folder WHERE folder_id = ?1 AND uid = ?2", params![m.from_folder_id, m.from_uid]).map_err(sql_error("mover"))?;
            tx.execute("UPDATE message SET in_transit_since = ?2 WHERE id = ?1", params![m.message_id, now()]).map_err(sql_error("mover"))?;
            tx.execute("UPDATE pending_move SET state = 'moved', moved_at = ?2 WHERE id = ?1", params![id, now()]).map_err(sql_error("mover"))?;
        } else {
            move_counts(&tx, m.message_id, m.from_folder_id, m.to_folder_id, true)?;
            tx.execute("DELETE FROM pending_move WHERE id = ?1", [id]).map_err(sql_error("mover"))?;
        }
    }
    tx.commit().map_err(sql_error("mover"))
}

/// Depois de uma sincronização do destino que começou em `synced_from`, os movimentos concluídos
/// antes dela já aparecem lá com o UID do servidor. Também limpa os que passaram do tempo.
pub fn settle_moves(conn: &Connection, to_folder_id: i64, synced_from: i64) -> Result<usize, AppError> {
    conn.execute(
        "DELETE FROM pending_move WHERE state = 'moved' AND ((to_folder_id = ?1 AND moved_at <= ?2) OR moved_at < ?3)",
        params![to_folder_id, synced_from, now() - IN_TRANSIT_SECS],
    )
    .map_err(sql_error("mover"))
}

/// Abertura do app: um MOVE interrompido volta a esperar e é refeito (o MOVE de um UID que já
/// saiu da pasta não faz nada no servidor).
pub fn resume_moves(conn: &Connection) -> Result<usize, AppError> {
    conn.execute("UPDATE pending_move SET state = 'waiting', due_at = ?1 WHERE state = 'running'", [now()]).map_err(sql_error("mover"))
}

/// Mensagens chegando a uma pasta que ainda não aparecem nela pelo servidor, das mais recentes
/// para as mais antigas movidas. O UID e a pasta são os da origem (o id que o frontend já tem).
pub fn incoming_moves(conn: &Connection, folder_id: i64) -> Result<Vec<(i64, ListedMessage)>, AppError> {
    let mut st = conn
        .prepare_cached(&format!(
            "SELECT {}, pm.from_folder_id
             FROM pending_move pm JOIN message m ON m.id = pm.message_id
             LEFT JOIN message_followup fu ON fu.message_id = m.id
             WHERE pm.to_folder_id = ?1
               AND NOT EXISTS (SELECT 1 FROM message_folder x WHERE x.message_id = m.id AND x.folder_id = ?1)
             ORDER BY pm.id DESC",
            LISTED_COLUMNS.replace("mf.uid", "pm.from_uid")
        ))
        .map_err(sql_error("mover"))?;
    let rows = st.query_map([folder_id], |r| Ok((r.get(14)?, listed_from(r)?))).map_err(sql_error("mover"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("mover"))
}

/// A mensagem de um id `(pasta, uid)`: pela ligação com a pasta ou, se ela já saiu da pasta num
/// mover deste computador, pelo registro do mover.
pub fn message_for_key(conn: &Connection, folder_id: i64, uid: u32) -> Result<Option<StoredMessage>, AppError> {
    if let Some(m) = message_by_uid(conn, folder_id, uid)? {
        return Ok(Some(m));
    }
    conn.query_row(
        "SELECT m.id, m.seen, m.flagged, m.body_state FROM pending_move pm JOIN message m ON m.id = pm.message_id
         WHERE pm.from_folder_id = ?1 AND pm.from_uid = ?2",
        params![folder_id, uid],
        |r| Ok(StoredMessage { id: r.get(0)?, flags: MessageFlags { seen: r.get(1)?, flagged: r.get(2)? }, body_state: r.get(3)? }),
    )
    .optional()
    .map_err(sql_error("mensagem"))
}

/// Onde a mensagem está no servidor agora: a ligação com alguma pasta, preferindo `prefer`.
pub fn location_of(conn: &Connection, message_id: i64, prefer: i64) -> Result<Option<(i64, u32)>, AppError> {
    conn.query_row(
        "SELECT folder_id, uid FROM message_folder WHERE message_id = ?1 ORDER BY folder_id <> ?2 LIMIT 1",
        params![message_id, prefer],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
    .map_err(sql_error("mensagem"))
}

pub fn folder_by_role(conn: &Connection, account_id: &str, role: Role) -> Result<Option<FolderRow>, AppError> {
    conn.query_row(&format!("{FOLDER_SELECT} WHERE account_id = ?1 AND role = ?2 ORDER BY position LIMIT 1"), params![account_id, role.as_str()], folder_from)
        .optional()
        .map_err(sql_error("pasta"))
}

pub fn touch_folder_usage(conn: &Connection, account_id: &str, folder_id: i64) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO folder_usage (account_id, folder_id, used_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (account_id, folder_id) DO UPDATE SET used_at = excluded.used_at",
        params![account_id, folder_id, now()],
    )
    .map_err(sql_error("pastas recentes"))?;
    Ok(())
}

/// As pastas para onde o usuário moveu mensagens mais recentemente (até `limit`).
pub fn recent_folders(conn: &Connection, account_id: &str, limit: u32) -> Result<Vec<FolderRow>, AppError> {
    let mut st = conn
        .prepare_cached(&format!(
            "{FOLDER_SELECT} WHERE id IN (SELECT folder_id FROM folder_usage WHERE account_id = ?1)
             ORDER BY (SELECT used_at FROM folder_usage u WHERE u.folder_id = folder.id) DESC, id DESC LIMIT ?2"
        ))
        .map_err(sql_error("pastas recentes"))?;
    let rows = st.query_map(params![account_id, limit], folder_from).map_err(sql_error("pastas recentes"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("pastas recentes"))
}

/// Nome de pasta nova: 1 a 100 caracteres, sem controle, sem "/" no começo ou no fim.
pub fn validate_folder_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    let count = name.chars().count();
    if count == 0 || count > 100 || name.chars().any(char::is_control) || name.starts_with('/') || name.ends_with('/') {
        return Err(AppError::InvalidInput);
    }
    Ok(name.to_owned())
}

pub fn folder_name_taken(conn: &Connection, account_id: &str, name: &str) -> Result<bool, AppError> {
    let folders = list_folders(conn, account_id)?;
    let wanted = name.to_lowercase();
    Ok(folders.iter().any(|f| f.display_name.to_lowercase() == wanted))
}

// ---------------------------------------------------------------- sinalização com prazo (CA7)

pub fn valid_due_date(date: &str) -> bool {
    let b = date.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |s: &str| s.parse::<u32>().ok();
    match (num(&date[0..4]), num(&date[5..7]), num(&date[8..10])) {
        (Some(y), Some(m), Some(d)) => (2000..=2999).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d),
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowupChange<'a> {
    Set(Option<&'a str>),
    Complete,
    Clear,
}

/// Grava o prazo localmente. A estrela no servidor é tratada por quem chama.
pub fn set_followup(conn: &Connection, message_id: i64, change: FollowupChange<'_>) -> Result<Option<Followup>, AppError> {
    match change {
        FollowupChange::Set(due) => {
            if due.is_some_and(|d| !valid_due_date(d)) {
                return Err(AppError::InvalidInput);
            }
            conn.execute(
                "INSERT INTO message_followup (message_id, due_date, completed_at) VALUES (?1, ?2, NULL)
                 ON CONFLICT (message_id) DO UPDATE SET due_date = excluded.due_date, completed_at = NULL",
                params![message_id, due],
            )
            .map_err(sql_error("sinalização"))?;
            Ok(Some(Followup { due: due.map(str::to_owned), completed: false }))
        }
        FollowupChange::Complete => {
            conn.execute(
                "INSERT INTO message_followup (message_id, due_date, completed_at) VALUES (?1, NULL, ?2)
                 ON CONFLICT (message_id) DO UPDATE SET completed_at = excluded.completed_at",
                params![message_id, now()],
            )
            .map_err(sql_error("sinalização"))?;
            let due: Option<String> = conn
                .query_row("SELECT due_date FROM message_followup WHERE message_id = ?1", [message_id], |r| r.get(0))
                .map_err(sql_error("sinalização"))?;
            Ok(Some(Followup { due, completed: true }))
        }
        FollowupChange::Clear => {
            conn.execute("DELETE FROM message_followup WHERE message_id = ?1", [message_id]).map_err(sql_error("sinalização"))?;
            Ok(None)
        }
    }
}

// ---------------------------------------------------------------- rascunhos locais (CA8)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftKind {
    New,
    Reply,
    ReplyAll,
    Forward,
}

impl DraftKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DraftKind::New => "new",
            DraftKind::Reply => "reply",
            DraftKind::ReplyAll => "reply_all",
            DraftKind::Forward => "forward",
        }
    }

    fn parse(value: &str) -> DraftKind {
        match value {
            "reply" => DraftKind::Reply,
            "reply_all" => DraftKind::ReplyAll,
            "forward" => DraftKind::Forward,
            _ => DraftKind::New,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct DraftContent {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
}

pub const MAX_DRAFT_BODY: usize = 1_000_000;
pub const MAX_RECIPIENTS: usize = 500;

impl DraftContent {
    pub fn validate(&self) -> Result<(), AppError> {
        let lists = [&self.to, &self.cc, &self.bcc];
        if self.body.len() > MAX_DRAFT_BODY
            || self.subject.chars().count() > 1000
            || lists.iter().map(|l| l.len()).sum::<usize>() > MAX_RECIPIENTS
            || lists.iter().any(|l| l.iter().any(|a| a.len() > 320))
        {
            return Err(AppError::InvalidInput);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub id: i64,
    pub kind: DraftKind,
    #[serde(flatten)]
    pub content: DraftContent,
    pub updated_at: i64,
}

fn draft_from(r: &Row<'_>) -> rusqlite::Result<Draft> {
    let list = |i: usize| -> rusqlite::Result<Vec<String>> { Ok(serde_json::from_str(&r.get::<_, String>(i)?).unwrap_or_default()) };
    Ok(Draft {
        id: r.get(0)?,
        kind: DraftKind::parse(&r.get::<_, String>(1)?),
        content: DraftContent { to: list(2)?, cc: list(3)?, bcc: list(4)?, subject: r.get(5)?, body: r.get(6)? },
        updated_at: r.get(7)?,
    })
}

const DRAFT_SELECT: &str = "SELECT id, kind, to_json, cc_json, bcc_json, subject, body, updated_at FROM draft";

pub fn get_draft(conn: &Connection, account_id: &str, kind: DraftKind, in_reply_to: Option<i64>) -> Result<Option<Draft>, AppError> {
    conn.query_row(
        &format!("{DRAFT_SELECT} WHERE account_id = ?1 AND kind = ?2 AND in_reply_to IS ?3"),
        params![account_id, kind.as_str(), in_reply_to],
        draft_from,
    )
    .optional()
    .map_err(sql_error("rascunho"))
}

/// Grava (ou atualiza) o rascunho: um por mensagem e tipo, e um "novo" por conta.
pub fn save_draft(conn: &Connection, account_id: &str, kind: DraftKind, in_reply_to: Option<i64>, content: &DraftContent) -> Result<Draft, AppError> {
    content.validate()?;
    if (kind == DraftKind::New) != in_reply_to.is_none() {
        return Err(AppError::InvalidInput);
    }
    let json = |l: &Vec<String>| serde_json::to_string(l).unwrap_or_else(|_| "[]".into());
    let existing = get_draft(conn, account_id, kind, in_reply_to)?;
    match existing {
        Some(d) => {
            conn.execute(
                "UPDATE draft SET to_json = ?2, cc_json = ?3, bcc_json = ?4, subject = ?5, body = ?6, updated_at = ?7 WHERE id = ?1",
                params![d.id, json(&content.to), json(&content.cc), json(&content.bcc), content.subject, content.body, now()],
            )
            .map_err(sql_error("rascunho"))?;
        }
        None => {
            conn.execute(
                "INSERT INTO draft (account_id, kind, in_reply_to, to_json, cc_json, bcc_json, subject, body, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![account_id, kind.as_str(), in_reply_to, json(&content.to), json(&content.cc), json(&content.bcc), content.subject, content.body, now()],
            )
            .map_err(sql_error("rascunho"))?;
        }
    }
    get_draft(conn, account_id, kind, in_reply_to)?.ok_or(AppError::Storage)
}

pub fn delete_draft(conn: &Connection, account_id: &str, kind: DraftKind, in_reply_to: Option<i64>) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM draft WHERE account_id = ?1 AND kind = ?2 AND in_reply_to IS ?3",
        params![account_id, kind.as_str(), in_reply_to],
    )
    .map_err(sql_error("rascunho"))?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::db::{new_key, Db};
    use crate::message::Person;
    use crate::providers;
    use proptest::prelude::*;

    pub struct TestDb {
        pub db: Db,
        pub path: std::path::PathBuf,
        _dir: tempfile::TempDir,
    }

    pub fn test_db() -> TestDb {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(crate::db::FILE_NAME);
        let db = Db::open(&path, &new_key()).unwrap();
        TestDb { db, path, _dir: dir }
    }

    pub fn gmail() -> &'static CatalogEntry {
        providers::find("gmail").unwrap()
    }

    pub fn account(c: &mut Connection, email: &str) -> Account {
        let id = new_account_id();
        let color = next_color(c).unwrap();
        insert_account(c, &id, gmail(), email, &default_display_name(email), color).unwrap()
    }

    pub fn folder(c: &mut Connection, account: &str, name: &str, role: Role) -> FolderRow {
        let mut remote: Vec<RemoteFolder> = list_folders(c, account)
            .unwrap()
            .into_iter()
            .map(|f| RemoteFolder { imap_name: f.imap_name, role: f.role, display_name: f.display_name, unread: f.unread, total: f.total })
            .collect();
        remote.push(RemoteFolder { imap_name: name.into(), role, display_name: name.into(), unread: Some(0), total: Some(0) });
        replace_folders(c, account, &remote).unwrap();
        list_folders(c, account).unwrap().into_iter().find(|f| f.imap_name == name).unwrap()
    }

    pub fn header(uid: u32, message_id: &str, subject: &str, seen: bool) -> RemoteHeader {
        RemoteHeader {
            uid,
            fields: HeaderFields {
                from_name: "Remetente".into(),
                from_address: "remetente@exemplo.com".into(),
                subject: subject.into(),
                date: Some(format!("2026-10-{:02}T10:00:00+00:00", 1 + uid % 28)),
                to: vec![Person { name: String::new(), address: "eu@exemplo.com".into() }],
                cc: vec![],
                message_id: Some(message_id.into()),
                in_reply_to: None,
                references: vec![],
                has_attachments: false,
            },
            internal_date: None,
            size: 1000,
            flags: MessageFlags { seen, flagged: false },
        }
    }

    fn snapshot(uids: std::ops::RangeInclusive<u32>) -> Snapshot {
        let headers: Vec<_> = uids.clone().map(|u| header(u, &format!("<{u}@x>"), "s", false)).collect();
        Snapshot {
            reset: true,
            uidvalidity: 1,
            uidnext: Some(uids.end() + 1),
            total: headers.len() as u32,
            unread: Some(headers.len() as u32),
            oldest_uid: Some(*uids.start()),
            complete: Some(*uids.start() == 1),
            headers,
            ..Default::default()
        }
    }

    fn count(c: &Connection, sql: &str) -> i64 {
        c.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn account_identity_is_unique_per_provider() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "Joao@Gmail.com");
            assert_eq!(a.email, "joao@gmail.com");
            assert_eq!(a.display_name, "Joao");
            assert_eq!(a.color, "blue");
            let dup = insert_account(c, &new_account_id(), gmail(), "joao@gmail.com", "Outra", "teal");
            assert_eq!(dup.err(), Some(AppError::Storage));
            assert_eq!(find_account(c, "gmail", "JOAO@gmail.com").unwrap().unwrap().id, a.id);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn colors_and_order_follow_the_rail() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "pessoal@gmail.com");
            let b = account(c, "trabalho@gmail.com");
            let f = account(c, "familia@gmail.com");
            assert_eq!([a.color.as_str(), b.color.as_str(), f.color.as_str()], ["blue", "teal", "green"]);
            let moved = move_account(c, &f.id, -1).unwrap();
            let ids: Vec<_> = moved.iter().map(|x| x.id.as_str()).collect();
            assert_eq!(ids, [a.id.as_str(), f.id.as_str(), b.id.as_str()]);
            let renamed = update_account(c, &b.id, Some("  Trabalho "), Some("purple")).unwrap();
            assert_eq!((renamed.display_name.as_str(), renamed.color.as_str()), ("Trabalho", "purple"));
            assert_eq!(update_account(c, &b.id, Some(""), None).err(), Some(AppError::InvalidInput));
            assert_eq!(update_account(c, &b.id, Some(&"x".repeat(33)), None).err(), Some(AppError::InvalidInput));
            assert_eq!(update_account(c, &b.id, None, Some("red")).err(), Some(AppError::InvalidInput));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn sign_out_keeps_every_row_and_reconnect_reuses_the_account() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "trabalho@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            apply_snapshot(c, inbox.id, &snapshot(1..=10)).unwrap();
            let before = count(c, "SELECT count(*) FROM message");
            set_status(c, &a.id, AccountStatus::SignedOut).unwrap();
            assert_eq!(count(c, "SELECT count(*) FROM message"), before);
            assert_eq!(list_page(c, inbox.id, None, 50).unwrap().len(), 10);
            assert_eq!(get_account(c, &a.id).unwrap().unwrap().status, AccountStatus::SignedOut);
            let again = find_account(c, "gmail", "trabalho@gmail.com").unwrap().unwrap();
            assert_eq!(again.id, a.id);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn message_in_two_folders_is_stored_once_and_leaves_with_the_last_link() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            let label = folder(c, &a.id, "Projetos", Role::Custom);
            let mut s1 = snapshot(1..=1);
            s1.headers = vec![header(5, "<mesma@x>", "Contrato", false)];
            let mut s2 = s1.clone();
            s2.headers = vec![header(90, "<mesma@x>", "Contrato", false)];
            apply_snapshot(c, inbox.id, &s1).unwrap();
            apply_snapshot(c, label.id, &s2).unwrap();
            assert_eq!(count(c, "SELECT count(*) FROM message"), 1);
            assert_eq!(count(c, "SELECT count(*) FROM message_folder"), 2);

            let gone = Snapshot { present: Some((1, vec![])), uidvalidity: 1, ..Default::default() };
            apply_snapshot(c, inbox.id, &gone).unwrap();
            assert_eq!(count(c, "SELECT count(*) FROM message"), 1, "ainda está no marcador");
            apply_snapshot(c, label.id, &gone).unwrap();
            assert_eq!(count(c, "SELECT count(*) FROM message"), 0);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn identical_copies_in_the_same_folder_stay_separate() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            let mut s = snapshot(1..=1);
            s.headers = vec![header(26, "<igual@x>", "Cópia", false), header(1, "<igual@x>", "Cópia", false)];
            apply_snapshot(c, inbox.id, &s).unwrap();
            assert_eq!(count(c, "SELECT count(*) FROM message"), 2);
            let first = message_by_uid(c, inbox.id, 26).unwrap().unwrap();
            set_local_flags(c, first.id, MessageFlags { seen: true, flagged: false }).unwrap();
            assert!(!message_by_uid(c, inbox.id, 1).unwrap().unwrap().flags.seen);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn search_escapes_wildcards_and_finds_by_subject_and_sender() {
        assert_eq!(like_pattern("50%_a\\b"), "%50\\%\\_a\\\\b%");
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let other = account(c, "outra@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            let label = folder(c, &a.id, "Projetos", Role::Custom);
            let other_inbox = folder(c, &other.id, "INBOX", Role::Inbox);
            let mut s1 = snapshot(1..=1);
            s1.headers = vec![header(1, "<a@x>", "Contrato Lumen 100%", false), header(2, "<b@x>", "Almoço", false)];
            apply_snapshot(c, inbox.id, &s1).unwrap();
            let mut s2 = snapshot(1..=1);
            s2.headers = vec![header(7, "<a@x>", "Contrato Lumen 100%", false)];
            apply_snapshot(c, label.id, &s2).unwrap();
            let mut s3 = snapshot(1..=1);
            s3.headers = vec![header(1, "<z@x>", "Contrato de outra conta", false)];
            apply_snapshot(c, other_inbox.id, &s3).unwrap();

            let hits = search(c, &a.id, "contrato").unwrap();
            assert_eq!(hits.len(), 1, "a mesma mensagem em duas pastas aparece uma vez, e só da conta pedida");
            assert_eq!((hits[0].folder_id, hits[0].message.uid), (inbox.id, 1));
            assert_eq!(search(c, &a.id, "100%").unwrap().len(), 1);
            assert_eq!(search(c, &a.id, "_").unwrap().len(), 0, "_ é literal");
            assert_eq!(search(c, &a.id, "remetente@exemplo").unwrap().len(), 2);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn purge_outside_keeps_the_list_and_drops_old_content() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            apply_snapshot(c, inbox.id, &snapshot(1..=3)).unwrap();
            for uid in 1..=3 {
                let m = message_by_uid(c, inbox.id, uid).unwrap().unwrap();
                save_body(c, m.id, b"conteudo", &[]).unwrap();
            }
            assert_eq!(stored_bytes_by_account(c).unwrap(), vec![(a.id.clone(), 24)]);
            assert_eq!(bytes_outside(c, Some("2026-10-04")).unwrap(), 16, "as de 2 e 3 de outubro");
            assert_eq!(purge_outside(c, Some("2026-10-04")).unwrap(), 2);
            assert_eq!(count(c, "SELECT count(*) FROM message"), 3, "a lista continua");
            assert_eq!(count(c, "SELECT count(*) FROM message_body"), 1);
            assert_eq!(count(c, "SELECT count(*) FROM message WHERE body_state = 'none'"), 2, "baixam de novo ao abrir");
            assert_eq!(bytes_outside(c, None).unwrap(), 8, "Só ao abrir: todo o resto fica de fora");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn only_signed_out_accounts_can_be_erased() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            apply_snapshot(c, inbox.id, &snapshot(1..=2)).unwrap();
            let m = message_by_uid(c, inbox.id, 1).unwrap().unwrap();
            save_body(c, m.id, b"conteudo", &[]).unwrap();
            assert!(matches!(delete_signed_out_account(c, &a.id), Err(AppError::InvalidInput)), "conta ativa: sair antes");
            set_status(c, &a.id, AccountStatus::SignedOut).unwrap();
            delete_signed_out_account(c, &a.id).unwrap();
            for table in ["account", "folder", "message", "message_body"] {
                assert_eq!(count(c, &format!("SELECT count(*) FROM {table}")), 0, "{table}");
            }
            compact(c).unwrap();
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn lock_settings_default_to_15_minutes_and_follow_windows() {
        let t = test_db();
        t.db.with(|c| {
            assert_eq!(lock_settings(c).unwrap(), LockSettings { minutes: "15".into(), with_windows: true });
            let s = LockSettings { minutes: "never".into(), with_windows: false };
            assert_eq!(set_lock_settings(c, &s).unwrap(), s);
            assert!(set_lock_settings(c, &LockSettings { minutes: "7".into(), with_windows: true }).is_err());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn moved_message_survives_until_the_target_sync_finds_it() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            let label = folder(c, &a.id, "Projetos", Role::Custom);
            apply_snapshot(c, inbox.id, &snapshot(1..=3)).unwrap();
            let m = message_by_uid(c, inbox.id, 2).unwrap().unwrap();
            save_body(c, m.id, b"corpo salvo", &[]).unwrap();
            set_followup(c, m.id, FollowupChange::Set(Some("2026-10-24"))).unwrap();

            assert_eq!(unlink(c, inbox.id, &[2], true).unwrap(), 1);
            let f = get_folder(c, inbox.id).unwrap().unwrap();
            assert_eq!((f.total, f.unread), (Some(2), Some(2)));
            assert_eq!(count(c, "SELECT count(*) FROM message"), 3, "em trânsito, não é apagada");

            let mut arrived = snapshot(1..=1);
            arrived.headers = vec![header(40, "<2@x>", "s", false)];
            arrived.reset = false;
            apply_snapshot(c, label.id, &arrived).unwrap();
            let same = message_by_uid(c, label.id, 40).unwrap().unwrap();
            assert_eq!(same.id, m.id);
            assert_eq!(load_body(c, m.id).unwrap().as_deref(), Some(&b"corpo salvo"[..]));
            let in_transit: Option<i64> = c.query_row("SELECT in_transit_since FROM message WHERE id = ?1", [m.id], |r| r.get(0)).unwrap();
            assert_eq!(in_transit, None);

            assert_eq!(unlink(c, inbox.id, &[3], false).unwrap(), 1);
            assert_eq!(count(c, "SELECT count(*) FROM message"), 2, "excluída de vez sai do banco");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn recent_folders_are_the_last_five_used_per_account() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let b = account(c, "b@gmail.com");
            let folders: Vec<_> = (0..7).map(|i| folder(c, &a.id, &format!("P{i}"), Role::Custom)).collect();
            let fb = folder(c, &b.id, "Outra", Role::Custom);
            for (i, f) in folders.iter().enumerate() {
                c.execute("INSERT INTO folder_usage (account_id, folder_id, used_at) VALUES (?1, ?2, ?3)", params![a.id, f.id, 100 + i as i64]).unwrap();
            }
            touch_folder_usage(c, &b.id, fb.id).unwrap();
            let names: Vec<_> = recent_folders(c, &a.id, 5).unwrap().into_iter().map(|f| f.display_name).collect();
            assert_eq!(names, ["P6", "P5", "P4", "P3", "P2"]);
            touch_folder_usage(c, &a.id, folders[0].id).unwrap();
            assert_eq!(recent_folders(c, &a.id, 5).unwrap()[0].display_name, "P0");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn folder_names_and_due_dates_are_validated() {
        assert_eq!(validate_folder_name("  Contratos 2026 ").unwrap(), "Contratos 2026");
        for bad in ["", "   ", "/raiz", "fim/", "a\nb", &"x".repeat(101)] {
            assert_eq!(validate_folder_name(bad), Err(AppError::InvalidInput), "{bad:?}");
        }
        assert!(valid_due_date("2026-10-24"));
        for bad in ["2026-13-01", "26-10-24", "2026/10/24", "2026-10-2a", "1999-01-01"] {
            assert!(!valid_due_date(bad), "{bad}");
        }
    }

    #[test]
    fn followup_is_dropped_when_the_star_disappears_elsewhere() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            let mut s = snapshot(1..=2);
            for h in &mut s.headers {
                h.flags.flagged = true;
            }
            apply_snapshot(c, inbox.id, &s).unwrap();
            let m1 = message_by_uid(c, inbox.id, 1).unwrap().unwrap();
            let m2 = message_by_uid(c, inbox.id, 2).unwrap().unwrap();
            set_followup(c, m1.id, FollowupChange::Set(None)).unwrap();
            set_followup(c, m2.id, FollowupChange::Set(Some("2026-10-24"))).unwrap();
            assert_eq!(set_followup(c, m2.id, FollowupChange::Set(Some("ontem"))).err(), Some(AppError::InvalidInput));
            let listed = list_page(c, inbox.id, None, 10).unwrap();
            assert_eq!(listed[0].followup, Some(Followup { due: Some("2026-10-24".into()), completed: false }));

            let unstar = Snapshot { uidvalidity: 1, flag_updates: vec![(1, MessageFlags { seen: false, flagged: false })], ..Default::default() };
            apply_snapshot(c, inbox.id, &unstar).unwrap();
            assert_eq!(count(c, "SELECT count(*) FROM message_followup"), 1);
            assert_eq!(set_followup(c, m2.id, FollowupChange::Complete).unwrap(), Some(Followup { due: Some("2026-10-24".into()), completed: true }));
            assert_eq!(set_followup(c, m2.id, FollowupChange::Clear).unwrap(), None);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn drafts_are_one_per_message_and_kind() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            apply_snapshot(c, inbox.id, &snapshot(1..=1)).unwrap();
            let m = message_by_uid(c, inbox.id, 1).unwrap().unwrap();
            let content = DraftContent { to: vec!["a@x.com".into()], cc: vec![], bcc: vec![], subject: "Re: s".into(), body: "oi".into() };
            let d1 = save_draft(c, &a.id, DraftKind::Reply, Some(m.id), &content).unwrap();
            let d2 = save_draft(c, &a.id, DraftKind::Reply, Some(m.id), &DraftContent { body: "oi de novo".into(), ..content.clone() }).unwrap();
            assert_eq!(d1.id, d2.id);
            assert_eq!(get_draft(c, &a.id, DraftKind::Reply, Some(m.id)).unwrap().unwrap().content.body, "oi de novo");
            save_draft(c, &a.id, DraftKind::Forward, Some(m.id), &content).unwrap();
            save_draft(c, &a.id, DraftKind::New, None, &content).unwrap();
            assert_eq!(save_draft(c, &a.id, DraftKind::New, Some(m.id), &content).err(), Some(AppError::InvalidInput));
            assert_eq!(count(c, "SELECT count(*) FROM draft"), 3);
            delete_draft(c, &a.id, DraftKind::Reply, Some(m.id)).unwrap();
            assert!(get_draft(c, &a.id, DraftKind::Reply, Some(m.id)).unwrap().is_none());
            let huge = DraftContent { body: "x".repeat(MAX_DRAFT_BODY + 1), ..content };
            assert_eq!(save_draft(c, &a.id, DraftKind::New, None, &huge).err(), Some(AppError::InvalidInput));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn list_width_is_saved_per_account_within_limits() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            set_list_width(c, &a.id, 480).unwrap();
            assert_eq!(get_account(c, &a.id).unwrap().unwrap().list_width, Some(480));
            assert_eq!(set_list_width(c, &a.id, 100).err(), Some(AppError::InvalidInput));
            assert_eq!(set_list_width(c, &a.id, 9000).err(), Some(AppError::InvalidInput));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn folder_of_another_account_is_rejected() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "a@gmail.com");
            let b = account(c, "b@gmail.com");
            let inbox_a = folder(c, &a.id, "INBOX", Role::Inbox);
            assert!(folder_of_account(c, &a.id, inbox_a.id).is_ok());
            assert_eq!(folder_of_account(c, &b.id, inbox_a.id).err(), Some(AppError::InvalidId));
            assert_eq!(folder_of_account(c, &a.id, 9999).err(), Some(AppError::InvalidId));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn uidvalidity_reset_drops_old_links() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            apply_snapshot(c, inbox.id, &snapshot(1..=5)).unwrap();
            let mut fresh = snapshot(100..=101);
            fresh.uidvalidity = 2;
            apply_snapshot(c, inbox.id, &fresh).unwrap();
            let uids: Vec<u32> = list_page(c, inbox.id, None, 50).unwrap().iter().map(|m| m.uid).collect();
            assert_eq!(uids, [101, 100]);
            let f = get_folder(c, inbox.id).unwrap().unwrap();
            assert_eq!((f.uidvalidity, f.oldest_uid, f.complete), (Some(2), Some(100), false));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn local_flags_adjust_unread_of_every_folder() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            apply_snapshot(c, inbox.id, &snapshot(1..=3)).unwrap();
            let m = message_by_uid(c, inbox.id, 2).unwrap().unwrap();
            set_local_flags(c, m.id, MessageFlags { seen: true, flagged: true }).unwrap();
            assert_eq!(get_folder(c, inbox.id).unwrap().unwrap().unread, Some(2));
            assert_eq!(get_account(c, &a.id).unwrap().unwrap().unread, Some(2));
            set_local_flags(c, m.id, MessageFlags { seen: true, flagged: false }).unwrap();
            assert_eq!(get_folder(c, inbox.id).unwrap().unwrap().unread, Some(2), "repetir não muda o contador");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn bodies_are_cached_with_attachments() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            apply_snapshot(c, inbox.id, &snapshot(1..=1)).unwrap();
            let m = message_by_uid(c, inbox.id, 1).unwrap().unwrap();
            assert_eq!(m.body_state, "none");
            let att = AttachmentInfo { name: "a.pdf".into(), mime_type: "application/pdf".into(), size: 3 };
            save_body(c, m.id, b"raw", &[att]).unwrap();
            assert_eq!(load_body(c, m.id).unwrap().as_deref(), Some(&b"raw"[..]));
            assert_eq!(message_by_uid(c, inbox.id, 1).unwrap().unwrap().body_state, "cached");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn database_file_has_no_plaintext() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "marcador.conta@gmail.com");
            let inbox = folder(c, &a.id, "PastaMarcadora", Role::Custom);
            let mut s = snapshot(1..=1);
            s.headers = vec![header(1, "<id@x>", "AssuntoMarcador", false)];
            apply_snapshot(c, inbox.id, &s).unwrap();
            let m = message_by_uid(c, inbox.id, 1).unwrap().unwrap();
            save_body(c, m.id, b"CorpoMarcador", &[]).unwrap();
            c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
            Ok(())
        })
        .unwrap();
        let mut bytes = std::fs::read(&t.path).unwrap();
        if let Ok(wal) = std::fs::read(format!("{}-wal", t.path.display())) {
            bytes.extend(wal);
        }
        let text = String::from_utf8_lossy(&bytes);
        assert!(!bytes.starts_with(b"SQLite format 3"));
        for marker in ["marcador.conta", "PastaMarcadora", "AssuntoMarcador", "CorpoMarcador", "remetente@exemplo"] {
            assert!(!text.contains(marker), "texto claro no arquivo: {marker}");
        }
    }

    /// Usado pelo teste de redaction de `message.rs`: passa por contas, pastas, mensagens e corpo
    /// com marcadores que nunca podem aparecer no log.
    pub fn exercise_for_redaction() {
        let t = test_db();
        t.db.with(|c| {
            let a = insert_account(c, &new_account_id(), gmail(), "redacao.marcador@gmail.com", "NomeContaMarcador", "teal").unwrap();
            let f = folder(c, &a.id, "PastaMarcadora", Role::Custom);
            let mut s = snapshot(1..=1);
            s.headers = vec![header(1, "<id@x>", "AssuntoMarcador", false)];
            apply_snapshot(c, f.id, &s).unwrap();
            let m = message_by_uid(c, f.id, 1).unwrap().unwrap();
            let att = AttachmentInfo { name: "AnexoMarcador.pdf".into(), mime_type: "application/pdf".into(), size: 1 };
            save_body(c, m.id, b"CorpoMarcador", &[att]).unwrap();
            set_local_flags(c, m.id, MessageFlags { seen: true, flagged: true }).unwrap();
            assert_eq!(search(c, &a.id, "AssuntoMarcador").unwrap().len(), 1);
            let draft = DraftContent { to: vec!["redacao.marcador@gmail.com".into()], cc: vec![], bcc: vec![], subject: "Re: AssuntoMarcador".into(), body: "CorpoMarcador".into() };
            save_draft(c, &a.id, DraftKind::Reply, Some(m.id), &draft).unwrap();
            set_followup(c, m.id, FollowupChange::Set(Some("2026-10-24"))).unwrap();
            unlink(c, f.id, &[1], true).unwrap();
            set_photo(c, &a.id, Some((b"FotoMarcador", "image/png"))).unwrap();
            set_account_interval(c, std::slice::from_ref(&a.id), Some(1), false).unwrap();
            set_default_interval(c, None).unwrap();
            set_status(c, &a.id, AccountStatus::SignedOut).unwrap();
            let _ = insert_account(c, &new_account_id(), gmail(), "redacao.marcador@gmail.com", "NomeContaMarcador", "teal");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn sync_intervals_default_per_account_and_bulk() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "a@gmail.com");
            let b = account(c, "b@gmail.com");
            let z = account(c, "z@gmail.com");
            let s = sync_settings(c).unwrap();
            assert_eq!(s.default_minutes, Some(SYNC_DEFAULT_MINUTES));
            assert!(s.accounts.iter().all(|x| x.uses_default && x.minutes == Some(5)));

            set_default_interval(c, Some(15)).unwrap();
            set_account_interval(c, &[a.id.clone(), b.id.clone()], Some(1), false).unwrap();
            set_account_interval(c, std::slice::from_ref(&z.id), None, false).unwrap();
            let s = sync_settings(c).unwrap();
            let of = |id: &str| s.accounts.iter().find(|x| x.account_id == id).unwrap().clone();
            assert_eq!((of(&a.id).minutes, of(&a.id).uses_default), (Some(1), false));
            assert_eq!((of(&b.id).minutes, of(&b.id).uses_default), (Some(1), false));
            assert_eq!((of(&z.id).minutes, of(&z.id).uses_default), (None, false));

            // Voltar ao padrão segue o padrão atual, inclusive "Manualmente".
            set_account_interval(c, std::slice::from_ref(&a.id), None, true).unwrap();
            set_default_interval(c, None).unwrap();
            let s = sync_settings(c).unwrap();
            let a_sync = s.accounts.iter().find(|x| x.account_id == a.id).unwrap();
            assert_eq!((a_sync.minutes, a_sync.uses_default), (None, true));

            for bad in [Some(0), Some(3), Some(61), Some(u32::MAX)] {
                assert_eq!(set_default_interval(c, bad), Err(AppError::InvalidInput));
                assert_eq!(set_account_interval(c, std::slice::from_ref(&a.id), bad, false), Err(AppError::InvalidInput));
            }
            assert_eq!(set_account_interval(c, &[], Some(5), false), Err(AppError::InvalidInput));
            // Conta inexistente desfaz o lote inteiro.
            assert_eq!(set_account_interval(c, &[b.id.clone(), new_account_id()], Some(30), false), Err(AppError::AccountNotFound));
            assert_eq!(sync_settings(c).unwrap().accounts.iter().find(|x| x.account_id == b.id).unwrap().minutes, Some(1));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn flagged_messages_by_bucket() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "a@gmail.com");
            let b = account(c, "b@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            let label = folder(c, &a.id, "Projetos", Role::Custom);
            let other = folder(c, &b.id, "INBOX", Role::Inbox);
            let mut s = snapshot(1..=7);
            s.headers = (1..=7).map(|u| header(u, &format!("<{u}@x>"), "s", true)).collect();
            apply_snapshot(c, inbox.id, &s).unwrap();
            // A mesma mensagem 1 também no marcador: conta uma vez.
            let mut l = snapshot(1..=1);
            l.headers = vec![header(1, "<1@x>", "s", true)];
            apply_snapshot(c, label.id, &l).unwrap();
            let mut o = snapshot(1..=1);
            o.headers = vec![header(1, "<o@x>", "s", true)];
            apply_snapshot(c, other.id, &o).unwrap();

            let flag = |c: &mut Connection, folder: i64, uid: u32, due: Option<&str>| {
                let m = message_by_uid(c, folder, uid).unwrap().unwrap();
                set_local_flags(c, m.id, MessageFlags { seen: true, flagged: true }).unwrap();
                set_followup(c, m.id, FollowupChange::Set(due)).unwrap();
                m.id
            };
            flag(c, inbox.id, 1, Some("2026-10-20")); // atrasada
            flag(c, inbox.id, 2, Some("2026-10-24")); // hoje
            flag(c, inbox.id, 3, Some("2026-10-25")); // esta semana
            flag(c, inbox.id, 4, Some("2026-11-02")); // mais tarde
            flag(c, inbox.id, 5, None); // sem data
            let done = flag(c, inbox.id, 6, Some("2026-10-20"));
            set_followup(c, done, FollowupChange::Complete).unwrap();
            // Prazo local, mas a estrela foi tirada: não conta.
            let off = flag(c, inbox.id, 7, None);
            set_local_flags(c, off, MessageFlags { seen: true, flagged: false }).unwrap();
            flag(c, other.id, 1, Some("2026-10-20")); // outra conta

            let dates = FlagDates { today: "2026-10-24".into(), week_end: "2026-10-30".into() };
            let counts = flagged_counts(c, &a.id, &dates).unwrap();
            assert_eq!(counts, FlaggedCounts { overdue: 1, today: 1, this_week: 1, later: 1, no_date: 1 });
            let overdue = flagged(c, &a.id, FlagBucket::Overdue, &dates).unwrap();
            assert_eq!(overdue.len(), 1);
            assert_eq!(overdue[0].folder_id, inbox.id);
            assert_eq!(flagged(c, &a.id, FlagBucket::NoDate, &dates).unwrap().len(), 1);
            assert_eq!(flagged_counts(c, &b.id, &dates).unwrap().overdue, 1);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn appearance_defaults_and_validation() {
        let t = test_db();
        let change = |theme: Option<&str>, canvas: Option<&str>| AppearanceChange { theme: theme.map(Into::into), canvas: canvas.map(Into::into), ..Default::default() };
        let look = |theme: &str, canvas: &str| Appearance { theme: theme.into(), canvas: canvas.into(), density: "standard".into(), show_folders: true, show_reader: true };
        t.db.with(|c| {
            assert_eq!(appearance(c).unwrap(), look("system", ""));
            assert_eq!(set_appearance(c, &change(Some("dark"), None)).unwrap(), look("dark", ""));
            assert_eq!(set_appearance(c, &change(None, Some("teal"))).unwrap(), look("dark", "teal"));
            assert_eq!(set_appearance(c, &change(Some("sepia"), None)), Err(AppError::InvalidInput));
            assert_eq!(set_appearance(c, &change(None, Some("url(http://x)"))), Err(AppError::InvalidInput));
            assert_eq!(set_appearance(c, &change(Some("light"), Some("x"))), Err(AppError::InvalidInput));
            assert_eq!(appearance(c).unwrap().theme, "dark");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn list_preferences_are_saved_and_validated() {
        let t = test_db();
        t.db.with(|c| {
            let saved = set_appearance(c, &AppearanceChange { density: Some("compact".into()), show_folders: Some(false), ..Default::default() }).unwrap();
            assert_eq!((saved.density.as_str(), saved.show_folders, saved.show_reader), ("compact", false, true));
            assert_eq!(set_appearance(c, &AppearanceChange { density: Some("dense".into()), ..Default::default() }), Err(AppError::InvalidInput));
            let saved = set_appearance(c, &AppearanceChange { show_folders: Some(true), show_reader: Some(false), ..Default::default() }).unwrap();
            assert_eq!((saved.density.as_str(), saved.show_folders, saved.show_reader), ("compact", true, false));
            Ok(())
        })
        .unwrap();
    }

    /// US7, D1: o e-mail sai da origem e entra no destino na hora, "Desfazer" devolve, e nenhuma
    /// recarga da origem o traz de volta enquanto o mover está pendente.
    #[test]
    fn pending_move_shows_in_target_and_hides_from_source() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            let label = folder(c, &a.id, "Projetos", Role::Custom);
            apply_snapshot(c, inbox.id, &snapshot(1..=3)).unwrap();
            apply_snapshot(c, label.id, &Snapshot { reset: true, uidvalidity: 7, total: 0, unread: Some(0), complete: Some(true), ..Default::default() }).unwrap();

            let accepted = create_pending_moves(c, &a.id, "lote1", &[(inbox.id, 2), (inbox.id, 99)], label.id, now() + 6).unwrap();
            assert_eq!(accepted, vec![(inbox.id, 2)], "uid inexistente fica de fora");
            let uids = |c: &Connection, f| list_page(c, f, None, 50).unwrap().into_iter().map(|m| m.uid).collect::<Vec<_>>();
            assert_eq!(uids(c, inbox.id), vec![3, 1]);
            let incoming = incoming_moves(c, label.id).unwrap();
            assert_eq!(incoming.len(), 1);
            assert_eq!((incoming[0].0, incoming[0].1.uid), (inbox.id, 2), "id da origem");
            let counts = |c: &Connection, f| get_folder(c, f).unwrap().map(|f| (f.total, f.unread)).unwrap();
            assert_eq!(counts(c, inbox.id), (Some(2), Some(2)));
            assert_eq!(counts(c, label.id), (Some(1), Some(1)));

            // Sincronizar a origem (o servidor ainda tem a mensagem) não a traz de volta.
            let mut again = snapshot(1..=3);
            again.reset = false;
            again.present = Some((1, vec![1, 2, 3]));
            apply_snapshot(c, inbox.id, &again).unwrap();
            assert_eq!(uids(c, inbox.id), vec![3, 1]);
            assert_eq!(counts(c, inbox.id), (Some(2), Some(2)), "contadores do servidor descontam o mover pendente");

            // O id da origem continua abrindo a mensagem.
            assert!(message_for_key(c, inbox.id, 2).unwrap().is_some());

            // Desfazer.
            assert_eq!(cancel_pending(c, "lote1").unwrap(), vec![(inbox.id, 2)]);
            assert_eq!(uids(c, inbox.id), vec![3, 2, 1]);
            assert!(incoming_moves(c, label.id).unwrap().is_empty());
            assert_eq!(counts(c, inbox.id), (Some(3), Some(3)));
            assert_eq!(counts(c, label.id), (Some(0), Some(0)));
            assert!(cancel_pending(c, "lote1").unwrap().is_empty());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn pending_move_runs_then_settles_after_the_target_sync() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            let label = folder(c, &a.id, "Projetos", Role::Custom);
            apply_snapshot(c, inbox.id, &snapshot(1..=3)).unwrap();
            let m = message_by_uid(c, inbox.id, 2).unwrap().unwrap();
            create_pending_moves(c, &a.id, "lote", &[(inbox.id, 2)], label.id, now() - 1).unwrap();
            create_pending_moves(c, &a.id, "outro", &[(inbox.id, 3)], label.id, now() + 60).unwrap();

            let due = take_due_moves(c, now()).unwrap();
            assert_eq!(due.iter().map(|d| d.from_uid).collect::<Vec<_>>(), vec![2], "só o vencido");
            assert!(cancel_pending(c, "lote").unwrap().is_empty(), "em andamento não desfaz");
            assert!(create_pending_moves(c, &a.id, "x", &[(inbox.id, 2)], inbox.id, now()).unwrap().is_empty());

            finish_moves(c, &[due[0].id], true).unwrap();
            assert!(message_by_uid(c, inbox.id, 2).unwrap().is_none(), "saiu da origem");
            assert_eq!(message_for_key(c, inbox.id, 2).unwrap().map(|s| s.id), Some(m.id), "o id antigo ainda resolve");
            assert_eq!(incoming_moves(c, label.id).unwrap().len(), 2, "continua aparecendo no destino");

            // A sincronização do destino encontra a mensagem com o UID de lá e o registro some.
            let started = now();
            let mut arrived = snapshot(1..=1);
            arrived.headers = vec![header(40, "<2@x>", "s", false)];
            apply_snapshot(c, label.id, &arrived).unwrap();
            assert_eq!(settle_moves(c, label.id, started).unwrap(), 1);
            let incoming = incoming_moves(c, label.id).unwrap();
            assert_eq!(incoming.iter().map(|(_, m)| m.uid).collect::<Vec<_>>(), vec![3], "a outra ainda espera");
            assert_eq!(message_by_uid(c, label.id, 40).unwrap().map(|s| s.id), Some(m.id));
            assert_eq!(location_of(c, m.id, inbox.id).unwrap(), Some((label.id, 40)));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn failed_or_retargeted_moves_put_things_back() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            let label = folder(c, &a.id, "Projetos", Role::Custom);
            let trash = folder(c, &a.id, "Lixeira", Role::Trash);
            apply_snapshot(c, inbox.id, &snapshot(1..=3)).unwrap();
            let total = |c: &Connection, f| get_folder(c, f).unwrap().unwrap().total;

            create_pending_moves(c, &a.id, "a", &[(inbox.id, 1)], label.id, now() + 60).unwrap();
            // Mover de novo antes do prazo troca o destino; mover de volta para a origem desfaz.
            assert_eq!(create_pending_moves(c, &a.id, "b", &[(inbox.id, 1)], trash.id, now() + 60).unwrap().len(), 1);
            assert!(incoming_moves(c, label.id).unwrap().is_empty());
            assert_eq!(incoming_moves(c, trash.id).unwrap().len(), 1);
            assert_eq!((total(c, inbox.id), total(c, label.id), total(c, trash.id)), (Some(2), Some(0), Some(1)));
            create_pending_moves(c, &a.id, "c", &[(inbox.id, 1)], inbox.id, now() + 60).unwrap();
            assert_eq!(count(c, "SELECT count(*) FROM pending_move"), 0);
            assert_eq!(total(c, inbox.id), Some(3));

            // Servidor recusou: volta para a origem.
            create_pending_moves(c, &a.id, "d", &[(inbox.id, 2)], label.id, now() - 1).unwrap();
            let due = take_due_moves(c, now()).unwrap();
            finish_moves(c, &[due[0].id], false).unwrap();
            assert!(message_by_uid(c, inbox.id, 2).unwrap().is_some());
            assert_eq!(list_page(c, inbox.id, None, 50).unwrap().len(), 3);
            assert_eq!((total(c, inbox.id), total(c, label.id)), (Some(3), Some(0)));

            // App fechado no meio do MOVE: volta a esperar.
            create_pending_moves(c, &a.id, "e", &[(inbox.id, 3)], label.id, now() - 1).unwrap();
            take_due_moves(c, now()).unwrap();
            assert_eq!(resume_moves(c).unwrap(), 1);
            assert_eq!(take_due_moves(c, now()).unwrap().len(), 1);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn previews_are_saved_and_listed() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            apply_snapshot(c, inbox.id, &snapshot(1..=3)).unwrap();
            assert_eq!(uids_without_preview(c, inbox.id, 2).unwrap(), vec![3, 2]);
            let p = crate::preview::Preview { snippet: "Oi, tudo bem?".into(), has_files: true, has_images: false };
            save_previews(c, inbox.id, &[(3, p), (2, crate::preview::Preview::default())]).unwrap();
            assert_eq!(uids_without_preview(c, inbox.id, 10).unwrap(), vec![1]);
            let page = list_page(c, inbox.id, None, 50).unwrap();
            assert_eq!((page[0].snippet.as_deref(), page[0].has_files, page[0].has_images), (Some("Oi, tudo bem?"), true, false));
            assert_eq!(page[1].snippet.as_deref(), Some(""), "sem texto: prévia vazia, não buscada de novo");
            assert_eq!(page[2].snippet, None);

            let m = message_by_uid(c, inbox.id, 1).unwrap().unwrap();
            save_body(c, m.id, b"Subject: s\r\n\r\nCorpo salvo.", &[]).unwrap();
            assert_eq!(list_page(c, inbox.id, None, 50).unwrap()[2].snippet.as_deref(), Some("Corpo salvo."));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn background_download_goes_folder_by_folder_newest_first() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            let trash = folder(c, &a.id, "[Gmail]/Lixeira", Role::Trash);
            let work = folder(c, &a.id, "Projetos", Role::Custom);
            apply_snapshot(c, inbox.id, &snapshot(1..=3)).unwrap();
            let mut other = snapshot(1..=2);
            other.headers = (1..=2).map(|u| header(u, &format!("<w{u}@x>"), "s", false)).collect();
            apply_snapshot(c, work.id, &other).unwrap();
            let mut gone = snapshot(1..=1);
            gone.headers = vec![header(1, "<t1@x>", "s", false)];
            apply_snapshot(c, trash.id, &gone).unwrap();

            let none = HashSet::new();
            let first = bodies_to_download(c, &a.id, None, 2_000_000, 25, &none).unwrap();
            assert_eq!(first.iter().map(|t| (t.folder_id, t.uid)).collect::<Vec<_>>(), [(inbox.id, 3), (inbox.id, 2), (inbox.id, 1)], "Caixa de entrada primeiro, mais nova antes");
            assert!(bodies_to_download(c, &a.id, None, 999, 25, &none).unwrap().is_empty(), "acima do tamanho fica para quando abrir");
            assert_eq!(bodies_to_download(c, &a.id, Some("2026-10-04"), 2_000_000, 25, &none).unwrap().len(), 1, "só a de 04/10 está no período");

            for t in &first {
                save_body(c, t.message_id, b"Subject: s\r\n\r\noi", &[]).unwrap();
            }
            let next = bodies_to_download(c, &a.id, None, 2_000_000, 1, &none).unwrap();
            assert_eq!(next.iter().map(|t| (t.folder_id, t.uid)).collect::<Vec<_>>(), [(work.id, 2)], "depois as outras pastas; a Lixeira nunca");
            let skip: HashSet<i64> = next.iter().map(|t| t.message_id).collect();
            assert_eq!(bodies_to_download(c, &a.id, None, 2_000_000, 5, &skip).unwrap().len(), 1, "a já tentada fica de fora");

            let asked = bodies_for_keys(c, &a.id, &[(inbox.id, 1), (work.id, 1), (work.id, 99)], 2_000_000).unwrap();
            assert_eq!(asked.iter().map(|t| t.uid).collect::<Vec<_>>(), [1], "só a que ainda não tem corpo e existe");
            assert_eq!(stored_in_folder(c, inbox.id).unwrap(), 3);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn offline_choice_becomes_a_start_date() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
        assert_eq!(offline_since("30", today), Some(Some("2026-09-25".into())));
        assert_eq!(offline_since("90", today), Some(Some("2026-07-27".into())));
        assert_eq!(offline_since("all", today), Some(None));
        assert_eq!(offline_since("none", today), None);
        let t = test_db();
        t.db.with(|c| {
            assert_eq!(offline_days(c).unwrap(), "30");
            assert_eq!(set_offline_days(c, "90").unwrap(), "90");
            assert_eq!(offline_days(c).unwrap(), "90");
            assert_eq!(set_offline_days(c, "7"), Err(AppError::InvalidInput));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn notices_keep_seven_days_newest_first() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let note = |title: &str| NewNotice {
                account_id: Some(a.id.clone()),
                kind: "mail".into(),
                title: title.into(),
                line1: "Contrato".into(),
                line2: String::new(),
                sender: Some("Marina".into()),
                target: Some(serde_json::json!({ "accountId": a.id })),
            };
            let day = 86_400;
            add_notices(c, &[note("velha")], 1_000_000).unwrap();
            add_notices(c, &[note("ontem"), note("hoje")], 1_000_000 + 7 * day).unwrap();
            let now = 1_000_000 + 8 * day;
            let list = list_notices(c, now).unwrap();
            assert_eq!(list.iter().map(|n| n.title.as_str()).collect::<Vec<_>>(), ["hoje", "ontem"], "a de 8 dias atrás sai");
            assert!(list.iter().all(|n| !n.seen) && list[0].target.is_some());
            mark_notices_seen(c).unwrap();
            assert!(list_notices(c, now).unwrap().iter().all(|n| n.seen));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn account_overview_counts_each_unread_message_once() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@gmail.com");
            let inbox = folder(c, &a.id, "INBOX", Role::Inbox);
            let all = folder(c, &a.id, "[Gmail]/Todos", Role::All);
            let label = folder(c, &a.id, "Projetos", Role::Custom);
            apply_snapshot(c, inbox.id, &snapshot(1..=3)).unwrap();
            let mut same = snapshot(1..=3);
            same.headers = (1..=3).map(|u| header(u + 10, &format!("<{u}@x>"), "s", false)).collect();
            apply_snapshot(c, all.id, &same).unwrap();
            let mut one = snapshot(1..=1);
            one.headers = vec![header(5, "<1@x>", "s", false)];
            apply_snapshot(c, label.id, &one).unwrap();
            let o = account_overview(c, &a.id).unwrap();
            assert_eq!((o.unread, o.saved), (3, 3));
            assert!(o.last_sync_at.is_some());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn account_photo_is_a_data_url_and_checked_daily() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "foto@gmail.com");
            assert_eq!(a.photo, None);
            assert!(photo_due(c, &a.id).unwrap());
            set_photo(c, &a.id, Some((&[1, 2, 3], "image/png"))).unwrap();
            assert_eq!(require_account(c, &a.id).unwrap().photo.as_deref(), Some("data:image/png;base64,AQID"));
            assert!(!photo_due(c, &a.id).unwrap());
            c.execute("UPDATE account SET photo_checked_at = ?2 WHERE id = ?1", params![a.id, now() - 25 * 3600]).unwrap();
            assert!(photo_due(c, &a.id).unwrap());
            set_photo(c, &a.id, None).unwrap();
            assert_eq!(require_account(c, &a.id).unwrap().photo, None);
            assert!(c.execute("UPDATE account SET photo_type = 'image/svg+xml' WHERE id = ?1", [&a.id]).is_err());
            assert_eq!(set_photo(c, &new_account_id(), None), Err(AppError::AccountNotFound));
            Ok(())
        })
        .unwrap();
    }

    #[derive(Debug, Clone)]
    enum Op {
        Sync { folder: usize, uid: u32, mid: u8, seen: bool },
        Remove { folder: usize, uid: u32 },
        Flag { folder: usize, uid: u32, seen: bool },
    }

    fn op() -> impl Strategy<Value = Op> {
        prop_oneof![
            (0..2usize, 1..40u32, 0..20u8, any::<bool>()).prop_map(|(folder, uid, mid, seen)| Op::Sync { folder, uid, mid, seen }),
            (0..2usize, 1..40u32).prop_map(|(folder, uid)| Op::Remove { folder, uid }),
            (0..2usize, 1..40u32, any::<bool>()).prop_map(|(folder, uid, seen)| Op::Flag { folder, uid, seen }),
        ]
    }

    fn unread_in(c: &Connection, folder_id: i64) -> i64 {
        count(c, &format!("SELECT count(*) FROM message_folder mf JOIN message m ON m.id = mf.message_id WHERE mf.folder_id = {folder_id} AND m.seen = 0"))
    }

    fn recount(c: &Connection, folders: &[FolderRow]) {
        for f in folders {
            c.execute("UPDATE folder SET unread = ?2 WHERE id = ?1", params![f.id, unread_in(c, f.id)]).unwrap();
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(48))]

        /// Qualquer sequência de sincronizar, remover e mudar flag deixa o banco consistente:
        /// nenhuma mensagem sem pasta, nenhuma ligação sem mensagem, e o contador de não lidas
        /// ajustado localmente (`set_local_flags`) igual à contagem real.
        #[test]
        fn random_operations_keep_the_database_consistent(ops in proptest::collection::vec(op(), 1..40)) {
            let t = test_db();
            t.db.with(|c| {
                let a = account(c, "prop@gmail.com");
                let folders = [folder(c, &a.id, "INBOX", Role::Inbox), folder(c, &a.id, "Outra", Role::Custom)];
                for f in &folders {
                    apply_snapshot(c, f.id, &Snapshot { reset: true, uidvalidity: 1, unread: Some(0), oldest_uid: Some(1), complete: Some(true), ..Default::default() }).unwrap();
                }
                for op in &ops {
                    match op {
                        Op::Sync { folder, uid, mid, seen } => {
                            let snap = Snapshot { uidvalidity: 1, unread: None, headers: vec![header(*uid, &format!("<{mid}@x>"), "s", *seen)], ..Default::default() };
                            apply_snapshot(c, folders[*folder].id, &snap).unwrap();
                            // Numa sincronização real o contador vem do servidor; aqui, da contagem real.
                            recount(c, &folders);
                        }
                        Op::Remove { folder, uid } => {
                            let f = &folders[*folder];
                            let present: Vec<u32> = uids_from(c, f.id, 1).unwrap().into_iter().filter(|u| u != uid).collect();
                            apply_snapshot(c, f.id, &Snapshot { uidvalidity: 1, unread: None, present: Some((1, present)), ..Default::default() }).unwrap();
                            recount(c, &folders);
                        }
                        Op::Flag { folder, uid, seen } => {
                            if let Some(m) = message_by_uid(c, folders[*folder].id, *uid).unwrap() {
                                set_local_flags(c, m.id, MessageFlags { seen: *seen, flagged: false }).unwrap();
                            }
                        }
                    }
                    assert_eq!(count(c, "SELECT count(*) FROM message m WHERE NOT EXISTS (SELECT 1 FROM message_folder mf WHERE mf.message_id = m.id)"), 0);
                    assert_eq!(count(c, "SELECT count(*) FROM message_folder mf WHERE NOT EXISTS (SELECT 1 FROM message m WHERE m.id = mf.message_id)"), 0);
                    for g in &folders {
                        let stored = count(c, &format!("SELECT unread FROM folder WHERE id = {}", g.id));
                        assert_eq!(stored, unread_in(c, g.id), "contador de não lidas divergiu");
                    }
                }
                Ok(())
            }).unwrap();
        }
    }
}
