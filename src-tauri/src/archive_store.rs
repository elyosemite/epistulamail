//! Arquivos locais no banco (US15): a "conta" sem servidor, as pastas de origem e a gravação das
//! mensagens importadas nas mesmas tabelas de uma conta (lista, leitura, conversas e busca valem
//! iguais). Mover e excluir num arquivo local acontecem só aqui, sem servidor.

use std::collections::{BTreeMap, HashMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::db::sql_error;
use crate::error::AppError;
use crate::mail_import::SourceFolder;
use crate::mailbox::Role;
use crate::message::{AttachmentInfo, HeaderFields, MessageFlags};
use crate::store::{self, now, Account, ARCHIVE_PROVIDER};

/// A pasta de Lixeira que todo arquivo local tem, para "Excluir" (a do backup, se houver, serve).
const TRASH_KEY: &str = "epistula:trash";
/// "Arquivar" num arquivo local sem "Todos os e-mails".
const ALL_KEY: &str = "epistula:all";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveRow {
    pub account_id: String,
    pub format: String,
    pub source_name: String,
    pub state: String,
    pub imported: u32,
    pub duplicates: u32,
    pub failures: BTreeMap<String, u32>,
    pub estimate: u32,
    pub created_at: i64,
    pub finished_at: Option<i64>,
    #[serde(skip)]
    pub source_path: String,
    #[serde(skip)]
    pub selected: Vec<String>,
}

const ARCHIVE_SELECT: &str = "SELECT account_id, format, source_name, state, imported, duplicates, failures, estimate, created_at, finished_at, source_path, selected FROM archive";

fn archive_from(r: &rusqlite::Row<'_>) -> rusqlite::Result<ArchiveRow> {
    Ok(ArchiveRow {
        account_id: r.get(0)?,
        format: r.get(1)?,
        source_name: r.get(2)?,
        state: r.get(3)?,
        imported: r.get(4)?,
        duplicates: r.get(5)?,
        failures: serde_json::from_str(&r.get::<_, String>(6)?).unwrap_or_default(),
        estimate: r.get(7)?,
        created_at: r.get(8)?,
        finished_at: r.get(9)?,
        source_path: r.get(10)?,
        selected: serde_json::from_str(&r.get::<_, String>(11)?).unwrap_or_default(),
    })
}

pub fn get_archive(conn: &Connection, account_id: &str) -> Result<Option<ArchiveRow>, AppError> {
    conn.query_row(&format!("{ARCHIVE_SELECT} WHERE account_id = ?1"), [account_id], archive_from)
        .optional()
        .map_err(sql_error("arquivo local"))
}

pub fn list_archives(conn: &Connection) -> Result<Vec<ArchiveRow>, AppError> {
    let mut st = conn.prepare_cached(&format!("{ARCHIVE_SELECT} ORDER BY created_at")).map_err(sql_error("arquivo local"))?;
    let rows = st.query_map([], archive_from).map_err(sql_error("arquivo local"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("arquivo local"))
}

/// O nome que aparece na coluna de contas: o pedido, ou o do arquivo, cortado em 32 caracteres.
pub fn archive_name(wanted: &str, source_name: &str) -> String {
    let base = if wanted.trim().is_empty() { source_name } else { wanted };
    let clean: String = base.trim().chars().filter(|c| !c.is_control()).take(32).collect();
    if clean.trim().is_empty() {
        "Arquivo".to_owned()
    } else {
        clean.trim().to_owned()
    }
}

pub struct NewArchive<'a> {
    pub name: &'a str,
    pub format: &'a str,
    pub source_name: &'a str,
    pub source_path: &'a str,
    pub estimate: u32,
}

/// Cria o arquivo local: a conta sem servidor, a linha de `archive` e as pastas escolhidas.
pub fn create_archive(conn: &mut Connection, id: &str, new: &NewArchive<'_>, folders: &[SourceFolder]) -> Result<Account, AppError> {
    let name = store::validate_name(&archive_name(new.name, new.source_name))?;
    let selected: Vec<&str> = folders.iter().map(|f| f.key.as_str()).collect();
    let tx = conn.transaction().map_err(sql_error("arquivo local"))?;
    tx.execute(
        "INSERT INTO account (id, provider, email, display_name, color, position, status, created_at)
         VALUES (?1, ?2, ?1, ?3, 'blue', (SELECT coalesce(max(position) + 1, 0) FROM account), 'signed_out', ?4)",
        params![id, ARCHIVE_PROVIDER, name, now()],
    )
    .map_err(sql_error("arquivo local"))?;
    tx.execute(
        "INSERT INTO archive (account_id, format, source_name, source_path, selected, state, estimate, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'importing', ?6, ?7)",
        params![id, new.format, new.source_name, new.source_path, serde_json::to_string(&selected).unwrap_or_default(), new.estimate, now()],
    )
    .map_err(sql_error("arquivo local"))?;
    add_folders(&tx, id, folders)?;
    tx.commit().map_err(sql_error("arquivo local"))?;
    tracing::info!(folders = folders.len(), "arquivo local criado");
    store::require_account(conn, id)
}

/// Pastas de origem que o arquivo local ainda não tem (a mesma chave junta as mensagens), e a Lixeira.
fn add_folders(conn: &Connection, account_id: &str, folders: &[SourceFolder]) -> Result<(), AppError> {
    let mut ordered: Vec<&SourceFolder> = folders.iter().collect();
    ordered.sort_by_key(|f| f.role != Role::Inbox);
    let mut insert = conn
        .prepare_cached(
            "INSERT INTO folder (account_id, imap_name, role, display_name, position, uidvalidity, uidnext, complete, total, unread, last_sync_at)
             VALUES (?1, ?2, ?3, ?4, (SELECT coalesce(max(position) + 1, 0) FROM folder WHERE account_id = ?1), 1, 1, 1, 0, 0, ?5)
             ON CONFLICT (account_id, imap_name) DO NOTHING",
        )
        .map_err(sql_error("pastas"))?;
    for f in ordered {
        insert.execute(params![account_id, f.key, f.role.as_str(), f.name, now()]).map_err(sql_error("pastas"))?;
    }
    if store::folder_by_role(conn, account_id, Role::Trash)?.is_none() {
        insert.execute(params![account_id, TRASH_KEY, Role::Trash.as_str(), "Lixeira", now()]).map_err(sql_error("pastas"))?;
    }
    Ok(())
}

/// "Importar mais neste arquivo": as pastas novas e a importação volta a correr.
pub fn reopen_archive(conn: &mut Connection, account_id: &str, new: &NewArchive<'_>, folders: &[SourceFolder]) -> Result<(), AppError> {
    let selected: Vec<&str> = folders.iter().map(|f| f.key.as_str()).collect();
    let tx = conn.transaction().map_err(sql_error("arquivo local"))?;
    add_folders(&tx, account_id, folders)?;
    tx.execute(
        "UPDATE archive SET format = ?2, source_name = ?3, source_path = ?4, selected = ?5, state = 'importing', estimate = estimate + ?6, finished_at = NULL
         WHERE account_id = ?1",
        params![account_id, new.format, new.source_name, new.source_path, serde_json::to_string(&selected).unwrap_or_default(), new.estimate],
    )
    .map_err(sql_error("arquivo local"))?;
    tx.commit().map_err(sql_error("arquivo local"))
}

/// Ids das pastas do arquivo local pela chave de origem.
pub fn folder_ids(conn: &Connection, account_id: &str) -> Result<HashMap<String, i64>, AppError> {
    let mut st = conn.prepare_cached("SELECT imap_name, id FROM folder WHERE account_id = ?1").map_err(sql_error("pastas"))?;
    let rows = st.query_map([account_id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(sql_error("pastas"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("pastas"))
}

pub fn set_progress(conn: &Connection, account_id: &str, imported: u32, duplicates: u32, failures: &BTreeMap<String, u32>) -> Result<(), AppError> {
    conn.execute(
        "UPDATE archive SET imported = ?2, duplicates = ?3, failures = ?4 WHERE account_id = ?1",
        params![account_id, imported, duplicates, serde_json::to_string(failures).unwrap_or_else(|_| "{}".into())],
    )
    .map_err(sql_error("arquivo local"))?;
    Ok(())
}

pub fn set_state(conn: &Connection, account_id: &str, state: &str) -> Result<(), AppError> {
    let finished = (state == "done").then(now);
    conn.execute("UPDATE archive SET state = ?2, finished_at = coalesce(?3, finished_at) WHERE account_id = ?1", params![account_id, state, finished])
        .map_err(sql_error("arquivo local"))?;
    Ok(())
}

/// Abertura do app: importação que estava correndo quando o app fechou vira "interrompida" (D5).
pub fn mark_interrupted(conn: &Connection) -> Result<usize, AppError> {
    conn.execute("UPDATE archive SET state = 'interrupted' WHERE state = 'importing'", []).map_err(sql_error("arquivo local"))
}

// ---------------------------------------------------------------- gravar

/// Uma mensagem lida e já interpretada (fora da conexão), pronta para gravar.
pub struct Imported {
    pub key: String,
    pub fields: HeaderFields,
    pub flags: MessageFlags,
    pub raw: Vec<u8>,
    pub attachments: Vec<AttachmentInfo>,
    pub preview: crate::preview::Preview,
    pub folders: Vec<i64>,
}

/// A mesma mensagem importada duas vezes: Message-ID e tamanho; sem Message-ID, o conteúdo.
pub fn dedupe_key(fields: &HeaderFields, raw: &[u8]) -> String {
    match &fields.message_id {
        Some(id) => format!("mid:{}:{}", crate::message::normalize_id(id), raw.len()),
        None => {
            let digest = Sha256::digest(raw);
            format!("sha:{}", digest.iter().map(|b| format!("{b:02x}")).collect::<String>())
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct BatchResult {
    pub imported: u32,
    pub duplicates: u32,
}

/// Grava um lote numa transação. Repetida é a mensagem que já está em todas as pastas pedidas.
pub fn import_batch(conn: &mut Connection, account_id: &str, items: &[Imported]) -> Result<BatchResult, AppError> {
    let tx = conn.transaction().map_err(sql_error("importação"))?;
    let mut result = BatchResult::default();
    let mut touched: HashSet<i64> = HashSet::new();
    {
        let mut find = tx.prepare_cached("SELECT id FROM message WHERE account_id = ?1 AND dedupe_key = ?2").map_err(sql_error("importação"))?;
        let mut linked = tx.prepare_cached("SELECT 1 FROM message_folder WHERE message_id = ?1 AND folder_id = ?2").map_err(sql_error("importação"))?;
        let mut next_uid = tx.prepare_cached("SELECT coalesce(max(uid), 0) + 1 FROM message_folder WHERE folder_id = ?1").map_err(sql_error("importação"))?;
        let mut link = tx.prepare_cached("INSERT INTO message_folder (message_id, folder_id, uid) VALUES (?1, ?2, ?3)").map_err(sql_error("importação"))?;
        let mut insert = tx
            .prepare_cached(
                "INSERT INTO message (account_id, dedupe_key, subject, from_name, from_address, to_json, cc_json, date, size, seen, flagged,
                    has_attachments, msgid, refs, thread_id, list_id, body_state, snippet, has_files, has_images)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, 'cached', ?17, ?18, ?19)
                 RETURNING id",
            )
            .map_err(sql_error("importação"))?;
        let mut body = tx.prepare_cached("INSERT INTO message_body (message_id, raw, fetched_at) VALUES (?1, ?2, ?3)").map_err(sql_error("importação"))?;
        let mut attachment = tx
            .prepare_cached("INSERT INTO attachment (message_id, name, mime_type, size) VALUES (?1, ?2, ?3, ?4)")
            .map_err(sql_error("importação"))?;
        for item in items {
            let existing: Option<i64> = find.query_row(params![account_id, item.key], |r| r.get(0)).optional().map_err(sql_error("importação"))?;
            let (id, is_new) = match existing {
                Some(id) => (id, false),
                None => {
                    let f = &item.fields;
                    let thread = store::thread_for(&tx, account_id, f)?;
                    let msgid = f.message_id.as_deref().map(crate::message::normalize_id);
                    let to = serde_json::to_string(&f.to).unwrap_or_else(|_| "[]".into());
                    let cc = serde_json::to_string(&f.cc).unwrap_or_else(|_| "[]".into());
                    let id: i64 = insert
                        .query_row(
                            params![
                                account_id,
                                item.key,
                                f.subject,
                                f.from_name,
                                f.from_address,
                                to,
                                cc,
                                f.date.clone().unwrap_or_default(),
                                item.raw.len() as i64,
                                item.flags.seen,
                                item.flags.flagged,
                                !item.attachments.is_empty(),
                                msgid,
                                store::refs_of(f),
                                thread,
                                f.list_id,
                                item.preview.snippet,
                                item.preview.has_files,
                                item.preview.has_images,
                            ],
                            |r| r.get(0),
                        )
                        .map_err(sql_error("importação"))?;
                    body.execute(params![id, item.raw, now()]).map_err(sql_error("importação"))?;
                    for a in &item.attachments {
                        attachment.execute(params![id, a.name, a.mime_type, a.size as i64]).map_err(sql_error("importação"))?;
                    }
                    (id, true)
                }
            };
            let mut added = false;
            for &folder in &item.folders {
                if linked.exists(params![id, folder]).map_err(sql_error("importação"))? {
                    continue;
                }
                let uid: i64 = next_uid.query_row([folder], |r| r.get(0)).map_err(sql_error("importação"))?;
                link.execute(params![id, folder, uid]).map_err(sql_error("importação"))?;
                touched.insert(folder);
                added = true;
            }
            if is_new || added {
                result.imported += 1;
            } else {
                result.duplicates += 1;
            }
        }
    }
    recount(&tx, &touched)?;
    tx.commit().map_err(sql_error("importação"))?;
    Ok(result)
}

/// Contadores das pastas pelo que está no banco (num arquivo local não há servidor para contar).
fn recount(conn: &Connection, folders: &HashSet<i64>) -> Result<(), AppError> {
    let mut st = conn
        .prepare_cached(
            "UPDATE folder SET
                total = (SELECT count(*) FROM message_folder WHERE folder_id = ?1),
                unread = (SELECT count(*) FROM message_folder mf JOIN message m ON m.id = mf.message_id WHERE mf.folder_id = ?1 AND m.seen = 0),
                uidnext = (SELECT coalesce(max(uid), 0) + 1 FROM message_folder WHERE folder_id = ?1)
             WHERE id = ?1",
        )
        .map_err(sql_error("pastas"))?;
    for id in folders {
        st.execute([id]).map_err(sql_error("pastas"))?;
    }
    Ok(())
}

fn timestamp(date: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(date).map(|d| d.timestamp()).unwrap_or(0)
}

/// No fim da importação, os UIDs de cada pasta seguem a data (a lista é ordenada pelo UID): um
/// backup raramente está em ordem. Pastas com um mover pendente ficam como estão.
pub fn order_by_date(conn: &mut Connection, account_id: &str) -> Result<(), AppError> {
    let tx = conn.transaction().map_err(sql_error("importação"))?;
    let folders: Vec<i64> = {
        let mut st = tx
            .prepare_cached(
                "SELECT id FROM folder f WHERE account_id = ?1
                 AND NOT EXISTS (SELECT 1 FROM pending_move pm WHERE pm.from_folder_id = f.id OR pm.to_folder_id = f.id)",
            )
            .map_err(sql_error("importação"))?;
        let rows = st.query_map([account_id], |r| r.get(0)).map_err(sql_error("importação"))?;
        rows.collect::<Result<_, _>>().map_err(sql_error("importação"))?
    };
    for folder in folders {
        let mut rows: Vec<(i64, i64, i64)> = {
            let mut st = tx
                .prepare_cached("SELECT mf.rowid, m.date, mf.uid FROM message_folder mf JOIN message m ON m.id = mf.message_id WHERE mf.folder_id = ?1")
                .map_err(sql_error("importação"))?;
            let found = st.query_map([folder], |r| Ok((r.get::<_, i64>(0)?, timestamp(&r.get::<_, String>(1)?), r.get::<_, i64>(2)?))).map_err(sql_error("importação"))?;
            found.collect::<Result<_, _>>().map_err(sql_error("importação"))?
        };
        rows.sort_by_key(|&(_, when, uid)| (when, uid));
        tx.execute("UPDATE message_folder SET uid = -uid WHERE folder_id = ?1", [folder]).map_err(sql_error("importação"))?;
        let mut set = tx.prepare_cached("UPDATE message_folder SET uid = ?2 WHERE rowid = ?1").map_err(sql_error("importação"))?;
        for (n, (rowid, _, _)) in rows.iter().enumerate() {
            set.execute(params![rowid, n as i64 + 1]).map_err(sql_error("importação"))?;
        }
        tx.execute("UPDATE folder SET uidnext = ?2 WHERE id = ?1", params![folder, rows.len() as i64 + 1]).map_err(sql_error("importação"))?;
    }
    tx.commit().map_err(sql_error("importação"))
}

// ---------------------------------------------------------------- mover e excluir sem servidor

/// Conclui, no banco, os movimentos vencidos de um arquivo local: a mensagem ganha um UID no destino
/// e sai da origem (os contadores já mudaram ao registrar o mover).
pub fn finish_local_moves(conn: &mut Connection, ids: &[i64]) -> Result<(), AppError> {
    let tx = conn.transaction().map_err(sql_error("mover"))?;
    for &id in ids {
        let row: Option<(i64, i64, i64, i64)> = tx
            .query_row("SELECT message_id, from_folder_id, from_uid, to_folder_id FROM pending_move WHERE id = ?1", [id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .optional()
            .map_err(sql_error("mover"))?;
        let Some((message_id, from, from_uid, to)) = row else { continue };
        let already: bool = tx
            .query_row("SELECT 1 FROM message_folder WHERE message_id = ?1 AND folder_id = ?2", params![message_id, to], |_| Ok(true))
            .optional()
            .map_err(sql_error("mover"))?
            .unwrap_or(false);
        if !already {
            let uid: i64 = tx.query_row("SELECT coalesce(max(uid), 0) + 1 FROM message_folder WHERE folder_id = ?1", [to], |r| r.get(0)).map_err(sql_error("mover"))?;
            tx.execute("INSERT INTO message_folder (message_id, folder_id, uid) VALUES (?1, ?2, ?3)", params![message_id, to, uid]).map_err(sql_error("mover"))?;
        }
        tx.execute("DELETE FROM message_folder WHERE folder_id = ?1 AND uid = ?2", params![from, from_uid]).map_err(sql_error("mover"))?;
        tx.execute("DELETE FROM pending_move WHERE id = ?1", [id]).map_err(sql_error("mover"))?;
        let both: HashSet<i64> = [from, to].into_iter().collect();
        recount(&tx, &both)?;
    }
    tx.commit().map_err(sql_error("mover"))
}

/// A pasta "Todos os e-mails" do arquivo local, criada na primeira vez que alguém arquiva.
pub fn ensure_all_folder(conn: &Connection, account_id: &str) -> Result<store::FolderRow, AppError> {
    if let Some(f) = store::folder_by_role(conn, account_id, Role::All)? {
        return Ok(f);
    }
    conn.execute(
        "INSERT INTO folder (account_id, imap_name, role, display_name, position, uidvalidity, uidnext, complete, total, unread, last_sync_at)
         VALUES (?1, ?2, 'all', 'Arquivados', (SELECT coalesce(max(position) + 1, 0) FROM folder WHERE account_id = ?1), 1, 1, 1, 0, 0, ?3)",
        params![account_id, ALL_KEY, now()],
    )
    .map_err(sql_error("pastas"))?;
    store::folder_by_role(conn, account_id, Role::All)?.ok_or(AppError::FolderNotFound)
}

/// Pasta nova num arquivo local (não há servidor para criar a pasta).
pub fn create_local_folder(conn: &Connection, account_id: &str, name: &str) -> Result<store::FolderRow, AppError> {
    let key = format!("epistula:{name}");
    conn.execute(
        "INSERT INTO folder (account_id, imap_name, role, display_name, position, uidvalidity, uidnext, complete, total, unread, last_sync_at)
         VALUES (?1, ?2, 'custom', ?3, (SELECT coalesce(max(position) + 1, 0) FROM folder WHERE account_id = ?1), 1, 1, 1, 0, 0, ?4)",
        params![account_id, key, name, now()],
    )
    .map_err(sql_error("pastas"))?;
    store::list_folders(conn, account_id)?.into_iter().find(|f| f.imap_name == key).ok_or(AppError::FolderNotFound)
}

/// Lida e estrela num arquivo local: só no banco.
pub fn set_flags(conn: &mut Connection, folder_id: i64, uids: &[u32], seen: Option<bool>, flagged: Option<bool>) -> Result<Vec<(u32, MessageFlags)>, AppError> {
    let mut out = Vec::new();
    for &uid in uids {
        if let Some(m) = store::message_by_uid(conn, folder_id, uid)? {
            let flags = MessageFlags { seen: seen.unwrap_or(m.flags.seen), flagged: flagged.unwrap_or(m.flags.flagged) };
            store::set_local_flags(conn, m.id, flags)?;
            out.push((uid, flags));
        }
    }
    let account: String = conn.query_row("SELECT account_id FROM folder WHERE id = ?1", [folder_id], |r| r.get(0)).map_err(sql_error("pastas"))?;
    let all: HashSet<i64> = folder_ids(conn, &account)?.into_values().collect();
    recount(conn, &all)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::parse_header_fields;
    use crate::store::tests::test_db;
    use mail_parser::MessageParser;

    fn item(raw: &str, folders: Vec<i64>, seen: bool) -> Imported {
        let raw = raw.as_bytes().to_vec();
        let fields = parse_header_fields(&MessageParser::default(), &raw).unwrap();
        Imported {
            key: dedupe_key(&fields, &raw),
            fields,
            flags: MessageFlags { seen, flagged: false },
            attachments: crate::message::attachments_of(&raw),
            preview: crate::preview::from_raw(&raw),
            raw,
            folders,
        }
    }

    fn source(key: &str, role: Role) -> SourceFolder {
        SourceFolder { key: key.into(), name: key.into(), role, depth: 0, count: 0, bytes: 0, label: false, selected: true }
    }

    const OLD: &str = "Message-ID: <a@x>\r\nFrom: Ana <ana@x.com>\r\nDate: Mon, 1 Jan 2024 10:00:00 +0000\r\nSubject: Antiga\r\n\r\nUm";
    const NEW: &str = "Message-ID: <b@x>\r\nIn-Reply-To: <a@x>\r\nFrom: Bia <bia@x.com>\r\nDate: Tue, 2 Jan 2024 10:00:00 +0000\r\nSubject: Re: Antiga\r\n\r\nDois";

    #[test]
    fn archive_is_local_and_imports_without_duplicates() {
        let t = test_db();
        t.db.with(|c| {
            let new = NewArchive { name: "", format: "mbox", source_name: "Gmail antigo.mbox", source_path: "C:/x/Gmail antigo.mbox", estimate: 2 };
            let account = create_archive(c, "11111111-2222-4333-8444-555555555555", &new, &[source("Inbox", Role::Inbox), source("Trabalho", Role::Custom)]).unwrap();
            assert_eq!(account.status, store::AccountStatus::Local);
            assert_eq!(account.display_name, "Gmail antigo.mbox");
            let ids = folder_ids(c, &account.id).unwrap();
            assert_eq!(ids.len(), 3, "as duas pastas e a Lixeira");
            let (inbox, work) = (ids["Inbox"], ids["Trabalho"]);

            // A mais nova vem antes no arquivo: no fim, a ordem da lista segue a data.
            let r = import_batch(c, &account.id, &[item(NEW, vec![inbox], false), item(OLD, vec![inbox, work], true)]).unwrap();
            assert_eq!(r, BatchResult { imported: 2, duplicates: 0 });
            let again = import_batch(c, &account.id, &[item(OLD, vec![inbox, work], true), item(NEW, vec![inbox, work], false)]).unwrap();
            assert_eq!(again, BatchResult { imported: 1, duplicates: 1 }, "a nova só ganhou a pasta Trabalho");
            order_by_date(c, &account.id).unwrap();
            let page = store::list_page(c, inbox, None, 10).unwrap();
            assert_eq!(page.len(), 1, "uma linha por conversa");
            assert_eq!(page[0].subject, "Re: Antiga");
            let inbox_row = store::get_folder(c, inbox).unwrap().unwrap();
            assert_eq!((inbox_row.total, inbox_row.unread, inbox_row.uidnext), (Some(2), Some(1), Some(3)));
            let first = store::message_by_uid(c, inbox, 1).unwrap().unwrap();
            assert!(store::load_body(c, first.id).unwrap().unwrap().ends_with(b"Um"), "a mais antiga ficou com o UID 1");

            // Lida e mover sem servidor.
            set_flags(c, inbox, &[2], Some(true), None).unwrap();
            assert_eq!(store::get_folder(c, inbox).unwrap().unwrap().unread, Some(0));
            let trash = store::folder_by_role(c, &account.id, Role::Trash).unwrap().unwrap();
            store::create_pending_moves(c, &account.id, "lote", &[(inbox, 1)], trash.id, 0).unwrap();
            let pending: Vec<i64> = store::take_due_moves(c, store::now()).unwrap().into_iter().map(|m| m.id).collect();
            finish_local_moves(c, &pending).unwrap();
            assert!(store::message_by_uid(c, inbox, 1).unwrap().is_none());
            assert_eq!(store::get_folder(c, trash.id).unwrap().unwrap().total, Some(1));
            assert_eq!(ensure_all_folder(c, &account.id).unwrap().display_name, "Arquivados");

            // O app fechou no meio de uma importação.
            assert_eq!(mark_interrupted(c).unwrap(), 1);
            assert_eq!(get_archive(c, &account.id).unwrap().unwrap().state, "interrupted");
            store::delete_signed_out_account(c, &account.id).unwrap();
            assert!(list_archives(c).unwrap().is_empty());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn messages_without_message_id_dedupe_by_content() {
        let raw = b"From: a@x.com\r\nSubject: s\r\n\r\nx";
        let fields = parse_header_fields(&MessageParser::default(), raw).unwrap();
        assert!(dedupe_key(&fields, raw).starts_with("sha:"));
        assert_ne!(dedupe_key(&fields, raw), dedupe_key(&fields, b"From: a@x.com\r\nSubject: s\r\n\r\ny"));
        assert_eq!(archive_name("  ", "takeout.mbox"), "takeout.mbox");
        assert_eq!(archive_name("Um nome bem comprido que passa de trinta e dois", "x").chars().count(), 32);
    }
}
