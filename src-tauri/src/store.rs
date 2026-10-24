//! Repositório do banco local (US3, CA2). Consultas sempre parametrizadas. Nenhuma função registra
//! em log endereço, nome de conta, nome de pasta, assunto, corpo ou nome de anexo.

use std::collections::HashSet;

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
}

const ACCOUNT_SELECT: &str = "SELECT a.id, a.provider, a.email, a.display_name, a.color, a.position, a.status,
        (SELECT f.unread FROM folder f WHERE f.account_id = a.id AND f.role = 'inbox' LIMIT 1),
        (SELECT s.value FROM account_setting s WHERE s.account_id = a.id AND s.key = 'last_folder'),
        (SELECT s.value FROM account_setting s WHERE s.account_id = a.id AND s.key = 'list_width')
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
                "INSERT INTO message (account_id, dedupe_key, subject, from_name, from_address, to_json, cc_json, date, size, seen, flagged, has_attachments)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT (account_id, dedupe_key) DO UPDATE SET seen = excluded.seen, flagged = excluded.flagged, in_transit_since = NULL
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
        for h in &snap.headers {
            let mut key = dedupe_key(&folder, snap.uidvalidity, h);
            if twin.exists(params![folder.account_id, key, folder_id, h.uid]).map_err(sql_error("sincronização"))? {
                key = uid_key(&folder, snap.uidvalidity, h.uid);
            }
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
                    ],
                    |r| r.get(0),
                )
                .map_err(sql_error("sincronização"))?;
            link.execute(params![id, folder_id, h.uid]).map_err(sql_error("sincronização"))?;
            report.upserted += 1;
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
    })
}

const LISTED_COLUMNS: &str = "mf.uid, m.subject, m.from_name, m.from_address, m.date, m.seen, m.flagged,
    fu.message_id IS NOT NULL, fu.due_date, fu.completed_at";

/// Página da pasta em ordem decrescente de UID, a partir de `before` (exclusivo).
pub fn list_page(conn: &Connection, folder_id: i64, before: Option<u32>, limit: u32) -> Result<Vec<ListedMessage>, AppError> {
    let mut st = conn
        .prepare_cached(
            &format!(
                "SELECT {LISTED_COLUMNS}
                 FROM message_folder mf JOIN message m ON m.id = mf.message_id
                 LEFT JOIN message_followup fu ON fu.message_id = m.id
                 WHERE mf.folder_id = ?1 AND mf.uid < ?2
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
    tx.execute(
        "UPDATE message SET body_state = 'cached', has_attachments = ?2 WHERE id = ?1",
        params![message_id, !attachments.is_empty()],
    )
    .map_err(sql_error("corpo"))?;
    tx.commit().map_err(sql_error("corpo"))
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
            Ok(SearchHit { message: listed_from(r)?, folder_id: r.get(10)?, folder_name: r.get(11)? })
        })
        .map_err(sql_error("busca"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("busca"))
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
            set_status(c, &a.id, AccountStatus::SignedOut).unwrap();
            let _ = insert_account(c, &new_account_id(), gmail(), "redacao.marcador@gmail.com", "NomeContaMarcador", "teal");
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
