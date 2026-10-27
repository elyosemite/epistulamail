//! Busca (US16): a consulta montada pelos chips (pessoas, período, filtros, pasta), numa conta ou em todas,
//! nas mensagens guardadas neste computador. Também as sugestões de pessoas, as buscas recentes e as salvas.
//! Nada do que é buscado vai para o log; os termos ficam no banco cifrado (recentes e salvas).

use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::sql_error;
use crate::error::AppError;
use crate::store::{self, ListedMessage, ARCHIVE_PROVIDER};

/// Resultados por busca: o bastante para achar, pouco para a tela.
pub const LIMIT: u32 = 200;
const MAX_TERM_CHARS: usize = 200;
const MAX_PEOPLE: usize = 10;
const MAX_RECENT: usize = 8;
const RECENT_KEY: &str = "recent_searches";

/// O que os chips da busca pedem. Pessoas do mesmo papel somam (uma ou outra); papéis diferentes, todos.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchQuery {
    /// Texto livre: cada palavra precisa aparecer no assunto, nas pessoas ou no começo do texto.
    pub text: String,
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    /// Em qualquer papel: De, Para ou Cc.
    pub with: Vec<String>,
    /// `AAAA-MM-DD`, inclusive.
    pub after: Option<String>,
    pub before: Option<String>,
    pub has_attachment: bool,
    pub unread: bool,
    pub starred: bool,
    /// Id opaco de uma pasta (`FolderKey`); só numa conta.
    pub folder: Option<String>,
    pub all_accounts: bool,
    pub oldest_first: bool,
}

fn valid_date(d: &str) -> bool {
    d.len() == 10 && chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok()
}

fn valid_term(t: &str) -> bool {
    t.chars().count() <= MAX_TERM_CHARS && !t.chars().any(char::is_control)
}

impl SearchQuery {
    /// Sem nenhum critério, não há o que buscar (o texto precisa de 2 letras).
    pub fn is_empty(&self) -> bool {
        self.text.trim().chars().count() < 2
            && self.from.is_empty()
            && self.to.is_empty()
            && self.cc.is_empty()
            && self.with.is_empty()
            && self.after.is_none()
            && self.before.is_none()
            && !self.has_attachment
            && !self.unread
            && !self.starred
            && self.folder.is_none()
    }

    pub fn validate(&self) -> Result<(), AppError> {
        let people = self.from.len() + self.to.len() + self.cc.len() + self.with.len();
        let terms_ok = std::iter::once(&self.text).chain(&self.from).chain(&self.to).chain(&self.cc).chain(&self.with).all(|t| valid_term(t));
        let dates_ok = self.after.as_deref().is_none_or(valid_date) && self.before.as_deref().is_none_or(valid_date);
        if people > MAX_PEOPLE || !terms_ok || !dates_ok || (self.folder.is_some() && self.all_accounts) {
            return Err(AppError::InvalidInput);
        }
        Ok(())
    }
}

/// Um resultado: a linha da lista, a conta, a pasta e o nome dela.
#[derive(Debug, Clone)]
pub struct Hit {
    pub account_id: String,
    pub folder_id: i64,
    pub folder_name: String,
    pub message: ListedMessage,
}

fn like(term: &str) -> Value {
    Value::Text(store::like_pattern(term.trim()))
}

/// Contas onde buscar: a pedida, ou todas as que aparecem na coluna (ativas, para reconectar e arquivos locais).
fn accounts_for(conn: &Connection, account_id: &str, all: bool) -> Result<Vec<String>, AppError> {
    if !all {
        store::require_account(conn, account_id)?;
        return Ok(vec![account_id.to_owned()]);
    }
    let mut st = conn
        .prepare_cached("SELECT id FROM account WHERE provider = ?1 OR status IN ('active', 'needs_reauth') ORDER BY position")
        .map_err(sql_error("busca"))?;
    let rows = st.query_map([ARCHIVE_PROVIDER], |r| r.get(0)).map_err(sql_error("busca"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("busca"))
}

/// `WHERE` da consulta e os valores, na ordem dos `?`.
fn conditions(conn: &Connection, q: &SearchQuery, accounts: &[String]) -> Result<(String, Vec<Value>), AppError> {
    let mut sql = String::new();
    let mut values: Vec<Value> = Vec::new();
    sql.push_str(&format!("m.account_id IN ({})", vec!["?"; accounts.len()].join(",")));
    values.extend(accounts.iter().map(|a| Value::Text(a.clone())));

    for word in q.text.split_whitespace().filter(|w| !w.is_empty()) {
        sql.push_str(
            " AND (m.subject LIKE ? ESCAPE '\\' OR m.from_name LIKE ? ESCAPE '\\' OR m.from_address LIKE ? ESCAPE '\\'
                   OR m.to_json LIKE ? ESCAPE '\\' OR m.cc_json LIKE ? ESCAPE '\\' OR m.snippet LIKE ? ESCAPE '\\')",
        );
        values.extend(std::iter::repeat_n(like(word), 6));
    }
    // Pessoas: nome ou endereço; no Para e no Cc, dentro da lista guardada em JSON.
    let mut people = |list: &[String], fields: &[&str]| {
        if list.is_empty() {
            return;
        }
        let mut any = Vec::new();
        for p in list {
            for f in fields {
                any.push(format!("{f} LIKE ? ESCAPE '\\'"));
                values.push(like(p));
            }
        }
        sql.push_str(&format!(" AND ({})", any.join(" OR ")));
    };
    people(&q.from, &["m.from_name", "m.from_address"]);
    people(&q.to, &["m.to_json"]);
    people(&q.cc, &["m.cc_json"]);
    people(&q.with, &["m.from_name", "m.from_address", "m.to_json", "m.cc_json"]);

    if let Some(d) = &q.after {
        sql.push_str(" AND substr(m.date, 1, 10) >= ?");
        values.push(Value::Text(d.clone()));
    }
    if let Some(d) = &q.before {
        sql.push_str(" AND substr(m.date, 1, 10) <= ?");
        values.push(Value::Text(d.clone()));
    }
    if q.has_attachment {
        sql.push_str(" AND (CASE WHEN m.snippet IS NULL THEN m.has_attachments ELSE m.has_files END) = 1");
    }
    if q.unread {
        sql.push_str(" AND m.seen = 0");
    }
    if q.starred {
        sql.push_str(" AND m.flagged = 1");
    }
    if let Some(key) = &q.folder {
        let k = crate::message::FolderKey::decode(key)?;
        if !accounts.contains(&k.account) {
            return Err(AppError::InvalidId);
        }
        store::folder_of_account(conn, &k.account, k.folder)?;
        sql.push_str(" AND EXISTS (SELECT 1 FROM message_folder x WHERE x.message_id = m.id AND x.folder_id = ?)");
        values.push(Value::Integer(k.folder));
    }
    Ok((sql, values))
}

/// A busca: até `LIMIT` + 1 resultados (o a mais diz que há mais), uma linha por mensagem.
pub fn search(conn: &Connection, account_id: &str, q: &SearchQuery) -> Result<Vec<Hit>, AppError> {
    q.validate()?;
    let accounts = accounts_for(conn, account_id, q.all_accounts)?;
    let (cond, mut values) = conditions(conn, q, &accounts)?;
    // Na busca por pasta, a linha mostra essa pasta; senão, a primeira pasta da mensagem.
    let folder_pick = match &q.folder {
        Some(key) => {
            values.push(Value::Integer(crate::message::FolderKey::decode(key)?.folder));
            "f.id = ?".to_owned()
        }
        None => "f.position = (SELECT min(f2.position) FROM message_folder mf2 JOIN folder f2 ON f2.id = mf2.folder_id WHERE mf2.message_id = m.id)".to_owned(),
    };
    values.push(Value::Integer(i64::from(LIMIT) + 1));
    let order = if q.oldest_first { "ASC" } else { "DESC" };
    let sql = format!(
        "SELECT {}, f.id, f.display_name, m.account_id
         FROM message m
         JOIN message_folder mf ON mf.message_id = m.id
         JOIN folder f ON f.id = mf.folder_id
         LEFT JOIN message_followup fu ON fu.message_id = m.id
         WHERE {cond} AND {folder_pick}
         GROUP BY m.id
         ORDER BY m.date {order} LIMIT ?",
        store::LISTED_COLUMNS
    );
    let mut st = conn.prepare(&sql).map_err(sql_error("busca"))?;
    let rows = st
        .query_map(params_from_iter(values), |r| {
            Ok(Hit { message: store::listed_row(r)?, folder_id: r.get(14)?, folder_name: r.get(15)?, account_id: r.get(16)? })
        })
        .map_err(sql_error("busca"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("busca"))
}

/// Quantas não lidas a busca acha (o número da busca salva na coluna de pastas).
pub fn unread_count(conn: &Connection, account_id: &str, q: &SearchQuery) -> Result<u32, AppError> {
    let q = SearchQuery { unread: true, ..q.clone() };
    q.validate()?;
    let accounts = accounts_for(conn, account_id, q.all_accounts)?;
    if accounts.is_empty() {
        return Ok(0);
    }
    let (cond, values) = conditions(conn, &q, &accounts)?;
    conn.query_row(&format!("SELECT count(*) FROM message m WHERE {cond}"), params_from_iter(values), |r| r.get(0))
        .map_err(sql_error("busca"))
}

// ---------------------------------------------------------------- pessoas (sugestões)

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub name: String,
    pub address: String,
    /// Em quantas mensagens a pessoa aparece (ajuda a escolher entre dois endereços).
    pub count: u32,
}

/// Pessoas que já escreveram ou receberam, pelo começo do nome ou do endereço, das que mais aparecem.
pub fn contacts(conn: &Connection, account_id: &str, all: bool, term: &str) -> Result<Vec<Contact>, AppError> {
    if !valid_term(term) {
        return Err(AppError::InvalidInput);
    }
    let accounts = accounts_for(conn, account_id, all)?;
    let ids = vec!["?"; accounts.len()].join(",");
    let sql = format!(
        "WITH people(name, address) AS (
            SELECT from_name, lower(from_address) FROM message WHERE account_id IN ({ids})
            UNION ALL SELECT json_extract(j.value, '$.name'), lower(json_extract(j.value, '$.address'))
                FROM message m, json_each(m.to_json) j WHERE m.account_id IN ({ids})
            UNION ALL SELECT json_extract(j.value, '$.name'), lower(json_extract(j.value, '$.address'))
                FROM message m, json_each(m.cc_json) j WHERE m.account_id IN ({ids})
         )
         SELECT coalesce(max(nullif(name, '')), ''), address, count(*) AS n FROM people
         WHERE address <> '' AND (address LIKE ?1x ESCAPE '\\' OR name LIKE ?2x ESCAPE '\\' OR name LIKE ?3x ESCAPE '\\')
         GROUP BY address ORDER BY n DESC LIMIT 8"
    );
    // Os três termos vão depois das contas (repetidas três vezes).
    let n = accounts.len() * 3;
    let sql = sql.replace("?1x", &format!("?{}", n + 1)).replace("?2x", &format!("?{}", n + 2)).replace("?3x", &format!("?{}", n + 3));
    let escaped = store::like_pattern(term.trim());
    let starts = format!("{}%", escaped.trim_start_matches('%').trim_end_matches('%'));
    let word = format!("% {starts}");
    let mut values: Vec<Value> = Vec::new();
    for _ in 0..3 {
        values.extend(accounts.iter().map(|a| Value::Text(a.clone())));
    }
    values.extend([Value::Text(starts.clone()), Value::Text(starts), Value::Text(word)]);
    let mut st = conn.prepare(&sql).map_err(sql_error("pessoas"))?;
    let rows = st
        .query_map(params_from_iter(values), |r| Ok(Contact { name: r.get(0)?, address: r.get(1)?, count: r.get(2)? }))
        .map_err(sql_error("pessoas"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("pessoas"))
}

// ---------------------------------------------------------------- recentes

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recent {
    /// Como a busca aparece na lista ("contrato · de Yuri Melo · com anexo").
    pub label: String,
    pub query: SearchQuery,
}

pub fn recent(conn: &Connection) -> Result<Vec<Recent>, AppError> {
    Ok(store::app_setting(conn, RECENT_KEY)?.and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default())
}

/// A busca entra no topo; a mesma busca não se repete; ficam as últimas `MAX_RECENT`.
pub fn add_recent(conn: &Connection, item: Recent) -> Result<Vec<Recent>, AppError> {
    item.query.validate()?;
    if item.label.trim().is_empty() || !valid_term(&item.label) {
        return Err(AppError::InvalidInput);
    }
    let mut list = recent(conn)?;
    list.retain(|r| r.query != item.query);
    list.insert(0, item);
    list.truncate(MAX_RECENT);
    store::set_app_setting(conn, RECENT_KEY, &serde_json::to_string(&list).unwrap_or_else(|_| "[]".into()))?;
    Ok(list)
}

pub fn clear_recent(conn: &Connection) -> Result<(), AppError> {
    store::set_app_setting(conn, RECENT_KEY, "[]")
}

// ---------------------------------------------------------------- salvas

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    pub id: i64,
    pub name: String,
    /// `None`: em todas as contas.
    pub account_id: Option<String>,
    pub query: SearchQuery,
    pub unread: u32,
}

/// As buscas salvas desta conta e as de todas as contas, com as não lidas de cada uma.
pub fn saved(conn: &Connection, account_id: &str) -> Result<Vec<Saved>, AppError> {
    let mut st = conn
        .prepare_cached("SELECT id, name, account_id, query FROM saved_search WHERE account_id = ?1 OR account_id IS NULL ORDER BY name COLLATE NOCASE")
        .map_err(sql_error("buscas salvas"))?;
    let rows: Vec<(i64, String, Option<String>, String)> = st
        .query_map([account_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .map_err(sql_error("buscas salvas"))?
        .collect::<Result<_, _>>()
        .map_err(sql_error("buscas salvas"))?;
    rows.into_iter()
        .filter_map(|(id, name, account, query)| serde_json::from_str::<SearchQuery>(&query).ok().map(|q| (id, name, account, q)))
        .map(|(id, name, account, query)| {
            let unread = unread_count(conn, account.as_deref().unwrap_or(account_id), &query).unwrap_or(0);
            Ok(Saved { id, name, account_id: account, query, unread })
        })
        .collect()
}

pub fn save(conn: &Connection, account_id: &str, name: &str, query: &SearchQuery) -> Result<i64, AppError> {
    query.validate()?;
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 60 || name.chars().any(char::is_control) || query.is_empty() {
        return Err(AppError::InvalidInput);
    }
    store::require_account(conn, account_id)?;
    let owner = (!query.all_accounts).then_some(account_id);
    conn.execute(
        "INSERT INTO saved_search (account_id, name, query, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![owner, name, serde_json::to_string(query).unwrap_or_default(), store::now()],
    )
    .map_err(sql_error("buscas salvas"))?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_saved(conn: &Connection, id: i64) -> Result<(), AppError> {
    let found: Option<i64> = conn.query_row("SELECT id FROM saved_search WHERE id = ?1", [id], |r| r.get(0)).optional().map_err(sql_error("buscas salvas"))?;
    found.ok_or(AppError::InvalidId)?;
    conn.execute("DELETE FROM saved_search WHERE id = ?1", [id]).map_err(sql_error("buscas salvas"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::{account, test_db};

    /// Uma mensagem de teste: pasta, uid, remetente (nome, endereço), destinatário, assunto, data, (lida, estrela), anexo.
    type Row<'a> = (&'a str, u32, (&'a str, &'a str), &'a str, &'a str, &'a str, (bool, bool), bool);

    fn insert(c: &mut Connection, account_id: &str, (folder, uid, from, to, subject, date, flags, files): Row) {
        c.execute(
            "INSERT INTO folder (account_id, imap_name, role, display_name, position, uidvalidity) VALUES (?1, ?2, 'custom', ?2, 0, 1)
             ON CONFLICT (account_id, imap_name) DO NOTHING",
            params![account_id, folder],
        )
        .unwrap();
        let folder_id: i64 = c.query_row("SELECT id FROM folder WHERE account_id = ?1 AND imap_name = ?2", params![account_id, folder], |r| r.get(0)).unwrap();
        let to_json = format!("[{{\"name\":\"\",\"address\":\"{to}\"}}]");
        c.execute(
            "INSERT INTO message (account_id, dedupe_key, subject, from_name, from_address, to_json, cc_json, date, size, seen, flagged, has_attachments, snippet, has_files)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, '[]', ?7, 10, ?8, ?9, ?10, '', ?10)",
            params![account_id, format!("k{uid}{account_id}"), subject, from.0, from.1, to_json, date, flags.0, flags.1, files],
        )
        .unwrap();
        let id = c.last_insert_rowid();
        c.execute("INSERT INTO message_folder (message_id, folder_id, uid) VALUES (?1, ?2, ?3)", params![id, folder_id, uid]).unwrap();
    }

    #[test]
    fn chips_filter_people_dates_and_flags() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@x.com");
            let b = account(c, "outra@x.com");
            insert(c, &a.id, ("Inbox", 1, ("Yuri Melo", "yuri@exemplo.com"), "eu@x.com", "Revisão do contrato", "2026-10-20T10:00:00+00:00", (false, false), true));
            insert(c, &a.id, ("Inbox", 2, ("Ana Ribeiro", "ana@lumen.com.br"), "yuri@exemplo.com", "Contrato assinado", "2026-09-02T10:00:00+00:00", (true, true), false));
            insert(c, &a.id, ("Jurídico", 3, ("Yuri Melo", "yuri@exemplo.com"), "eu@x.com", "Minuta antiga", "2015-03-01T10:00:00+00:00", (true, false), true));
            insert(c, &b.id, ("Inbox", 4, ("Yuri Melo", "yuri@exemplo.com"), "outra@x.com", "Contrato da outra conta", "2026-10-21T10:00:00+00:00", (false, false), false));

            let q = |q: SearchQuery| search(c, &a.id, &q).unwrap().into_iter().map(|h| h.message.subject).collect::<Vec<_>>();
            assert_eq!(q(SearchQuery { from: vec!["Yuri Melo".into()], ..Default::default() }), ["Revisão do contrato", "Minuta antiga"]);
            assert_eq!(q(SearchQuery { to: vec!["yuri@exemplo.com".into()], ..Default::default() }), ["Contrato assinado"], "o destinatário pelo endereço");
            assert_eq!(q(SearchQuery { with: vec!["yuri".into()], ..Default::default() }).len(), 3, "em qualquer papel");
            assert_eq!(q(SearchQuery { from: vec!["yuri".into(), "ana@lumen".into()], ..Default::default() }).len(), 3, "duas pessoas do mesmo papel somam");
            assert_eq!(q(SearchQuery { from: vec!["yuri".into()], after: Some("2026-01-01".into()), ..Default::default() }), ["Revisão do contrato"]);
            assert_eq!(q(SearchQuery { before: Some("2015-12-31".into()), ..Default::default() }), ["Minuta antiga"]);
            assert_eq!(q(SearchQuery { has_attachment: true, unread: true, ..Default::default() }), ["Revisão do contrato"]);
            assert_eq!(q(SearchQuery { starred: true, ..Default::default() }), ["Contrato assinado"]);
            assert_eq!(q(SearchQuery { text: "contrato revisão".into(), ..Default::default() }), ["Revisão do contrato"], "todas as palavras");
            assert_eq!(q(SearchQuery { text: "contrato".into(), oldest_first: true, ..Default::default() }), ["Contrato assinado", "Revisão do contrato"]);

            let all = search(c, &a.id, &SearchQuery { text: "contrato".into(), all_accounts: true, ..Default::default() }).unwrap();
            assert_eq!(all.len(), 3);
            assert_eq!(all[0].account_id, b.id, "a outra conta entra em todas as contas");

            let juridico = store::list_folders(c, &a.id).unwrap().into_iter().find(|f| f.display_name == "Jurídico").unwrap();
            let key = crate::message::FolderKey { account: a.id.clone(), folder: juridico.id }.encode();
            let hits = search(c, &a.id, &SearchQuery { folder: Some(key.clone()), ..Default::default() }).unwrap();
            assert_eq!((hits.len(), hits[0].folder_name.as_str()), (1, "Jurídico"));
            assert!(search(c, &a.id, &SearchQuery { folder: Some(key), all_accounts: true, ..Default::default() }).is_err(), "pasta só numa conta");
            assert!(search(c, &a.id, &SearchQuery { after: Some("31/12/2026".into()), ..Default::default() }).is_err());
            assert!(SearchQuery { text: "a".into(), ..Default::default() }.is_empty());

            let people = contacts(c, &a.id, false, "yu").unwrap();
            assert_eq!(people[0], Contact { name: "Yuri Melo".into(), address: "yuri@exemplo.com".into(), count: 3 }, "de e para somam");
            assert!(contacts(c, &a.id, false, "melo").unwrap().iter().any(|p| p.address == "yuri@exemplo.com"), "pelo sobrenome");
            assert_eq!(unread_count(c, &a.id, &SearchQuery { from: vec!["yuri".into()], ..Default::default() }).unwrap(), 1);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn recent_and_saved_searches() {
        let t = test_db();
        t.db.with(|c| {
            let a = account(c, "eu@x.com");
            let q = SearchQuery { text: "contrato".into(), ..Default::default() };
            for i in 0..10 {
                add_recent(c, Recent { label: format!("busca {i}"), query: SearchQuery { text: format!("termo {i}"), ..Default::default() } }).unwrap();
            }
            add_recent(c, Recent { label: "contrato".into(), query: q.clone() }).unwrap();
            let list = add_recent(c, Recent { label: "contrato".into(), query: q.clone() }).unwrap();
            assert_eq!(list.len(), MAX_RECENT);
            assert_eq!(list.iter().filter(|r| r.query == q).count(), 1, "a mesma busca não se repete");
            clear_recent(c).unwrap();
            assert!(recent(c).unwrap().is_empty());

            let id = save(c, &a.id, "Contratos", &q).unwrap();
            save(c, &a.id, "Em todas", &SearchQuery { all_accounts: true, ..q.clone() }).unwrap();
            let saved_list = saved(c, &a.id).unwrap();
            assert_eq!(saved_list.iter().map(|s| (s.name.as_str(), s.account_id.is_some())).collect::<Vec<_>>(), [("Contratos", true), ("Em todas", false)]);
            assert!(save(c, &a.id, "Vazia", &SearchQuery::default()).is_err());
            delete_saved(c, id).unwrap();
            assert_eq!(saved(c, &a.id).unwrap().len(), 1);
            Ok(())
        })
        .unwrap();
    }
}
