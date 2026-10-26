//! Execução das regras (US12): ao sincronizar (CA3), em "Rodar agora" (CA4), na limpeza diária
//! (CA8), na prévia do editor (CA2) e no desfazer do histórico (CA6). O que a regra decide vem de
//! `rules.rs`; aqui ficam o banco, o servidor e o histórico. Mover, arquivar e excluir usam a fila do
//! mover (US7), que continua depois de fechar o app. Logs: só quantidades e tipos de ação.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Duration;

use chrono::Local;
use futures::FutureExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppError;
use crate::imap_client;
use crate::mailbox::Role;
use crate::message::{FolderKey, MessageFlags};
use crate::rules::{self, Action, Facts, Notify, Plan, Rule, RuleSpec};
use crate::session::{AppState, Lane};
use crate::store::{self, Account, AccountStatus, FolderRow, RuleHit};

/// A prévia olha os e-mails mais recentes da pasta (CA2).
pub const PREVIEW_WINDOW: u32 = 500;
const PREVIEW_SAMPLE: usize = 50;
/// Lotes maiores que isso também vão para o sino (CA6).
const BELL_BATCH: usize = 20;
/// De quanto em quanto tempo conferir se a limpeza do dia já rodou.
const CLEANUP_TICK: Duration = Duration::from_secs(3600);

/// Pastas onde regras nunca rodam (D2).
fn never_runs_in(role: Role) -> bool {
    matches!(role, Role::Spam | Role::Trash | Role::Sent | Role::Drafts)
}

/// A regra roda nesta pasta ao sincronizar? Caixa de entrada sempre; as outras, se a regra pedir.
fn runs_in(spec: &RuleSpec, account_id: &str, folder: &FolderRow) -> bool {
    if never_runs_in(folder.role) || spec.is_cleanup() {
        return false;
    }
    folder.role == Role::Inbox || spec.folders.contains(&FolderKey { account: account_id.to_owned(), folder: folder.id }.encode())
}

/// Para as notificações (US8): o que não vira "e-mail novo" e o que avisa sempre.
#[derive(Debug, Default)]
pub struct NewMailOutcome {
    pub silent: HashSet<i64>,
    pub always: HashSet<i64>,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Summary {
    moved: usize,
    archived: usize,
    deleted: usize,
    copied: usize,
    read: usize,
    starred: usize,
    followup: usize,
    /// Nome da pasta de destino de "Mover para" ou do marcador, para o histórico.
    #[serde(skip_serializing_if = "Option::is_none")]
    folder: Option<String>,
}

/// Aviso para o toast depois de uma sincronização em que regras agiram (CA6).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RulesApplied {
    account_id: String,
    count: usize,
    moved: usize,
    archived: usize,
    deleted: usize,
    run_ids: Vec<i64>,
}

/// Resultado de uma execução: os lotes gravados e os totais.
#[derive(Debug, Default)]
struct Executed {
    run_ids: Vec<i64>,
    messages: HashSet<i64>,
    total: Summary,
}

/// E-mails novos de uma pasta passam pelas regras logo depois da sincronização (CA3).
pub async fn on_new_mail(state: &AppState, account_id: &str, folder_id: i64, new_ids: &[i64]) -> NewMailOutcome {
    let mut outcome = NewMailOutcome::default();
    if new_ids.is_empty() {
        return outcome;
    }
    let (id, ids) = (account_id.to_owned(), new_ids.to_vec());
    let today = Local::now().date_naive();
    let loaded = state
        .db
        .call(move |c| {
            let account = store::require_account(c, &id)?;
            let folder = store::get_folder(c, folder_id)?.ok_or(AppError::FolderNotFound)?;
            let rules: Vec<Rule> = store::active_rules(c, &id)?.into_iter().filter(|r| runs_in(&r.spec, &id, &folder)).collect();
            let facts = if rules.is_empty() { Vec::new() } else { store::rule_facts(c, folder_id, Some(&ids), None, today)? };
            Ok((account, folder, rules, facts))
        })
        .await;
    let Ok((account, folder, rules, facts)) = loaded else { return outcome };
    if rules.is_empty() || facts.is_empty() || account.status != AccountStatus::Active {
        return outcome;
    }
    let items: Vec<(Facts, Plan)> = facts.into_iter().map(|f| {
        let p = rules::plan(&rules, &f, &account.email);
        (f, p)
    }).collect();
    for (f, p) in &items {
        match p.notify {
            Notify::Silent => {
                outcome.silent.insert(f.message_id);
            }
            Notify::Always => {
                outcome.always.insert(f.message_id);
            }
            Notify::Normal => {}
        }
    }
    match execute(state, &account, &folder, &rules, items, "sync").await {
        Ok(done) if !done.messages.is_empty() => {
            let applied = RulesApplied {
                account_id: account.id.clone(),
                count: done.messages.len(),
                moved: done.total.moved,
                archived: done.total.archived,
                deleted: done.total.deleted,
                run_ids: done.run_ids,
            };
            tracing::info!(count = applied.count, "regras aplicadas na sincronização");
            if applied.count > BELL_BATCH {
                crate::notify::bell(&state.app, &account.id, "Regras organizaram vários e-mails", format!("{} e-mails em {}", applied.count, account.display_name));
            }
            let _ = state.app.emit("rules-applied", applied);
        }
        Ok(_) => {}
        Err(e) => tracing::warn!(kind = e.kind(), "regras não aplicadas nesta sincronização"),
    }
    outcome
}

/// Pasta de destino de uma ação, ou `None` quando ela não existe mais.
fn resolve_target(c: &rusqlite::Connection, account_id: &str, action: &Action) -> Result<Option<FolderRow>, AppError> {
    let by_key = |key: &str| -> Result<Option<FolderRow>, AppError> {
        match FolderKey::decode(key) {
            Ok(k) if k.account == account_id => Ok(store::get_folder(c, k.folder)?.filter(|f| f.account_id == account_id && f.role != Role::Starred)),
            _ => Ok(None),
        }
    };
    match action {
        Action::Move { folder } | Action::Copy { folder } => by_key(folder),
        Action::Archive => store::folder_by_role(c, account_id, Role::All),
        Action::Delete => store::folder_by_role(c, account_id, Role::Trash),
        _ => Ok(None),
    }
}

fn due_date(days: u8) -> String {
    (Local::now().date_naive() + chrono::Duration::days(days as i64)).format("%Y-%m-%d").to_string()
}

/// Executa os planos de e-mails de uma pasta: flags e marcadores no servidor, prazos e movimentos
/// no banco (a fila do mover leva ao servidor) e um lote no histórico por regra.
async fn execute(state: &AppState, account: &Account, folder: &FolderRow, rules: &[Rule], items: Vec<(Facts, Plan)>, origin: &str) -> Result<Executed, AppError> {
    let mut done = Executed::default();
    if items.iter().all(|(_, p)| p.steps.is_empty()) {
        return Ok(done);
    }
    // Destinos de cada ação de pasta, uma vez por regra. Regra com destino sumido é desligada (CA9).
    let (account_id, rules_owned) = (account.id.clone(), rules.to_vec());
    let targets: HashMap<(i64, usize), FolderRow> = state
        .db
        .call(move |c| {
            let mut out = HashMap::new();
            for r in &rules_owned {
                for (i, a) in r.spec.actions.iter().enumerate() {
                    if !matches!(a, Action::Move { .. } | Action::Copy { .. } | Action::Archive | Action::Delete) {
                        continue;
                    }
                    match resolve_target(c, &account_id, a)? {
                        Some(f) => {
                            out.insert((r.id, i), f);
                        }
                        None => {
                            store::set_rule_problem(c, r.id, "folder_missing")?;
                            tracing::warn!("regra desligada: pasta de destino não existe mais");
                        }
                    }
                }
            }
            Ok(out)
        })
        .await?;
    let broken: HashSet<i64> = rules.iter().filter(|r| r.spec.actions.iter().enumerate().any(|(i, a)| matches!(a, Action::Move { .. } | Action::Copy { .. } | Action::Archive | Action::Delete) && !targets.contains_key(&(r.id, i)))).map(|r| r.id).collect();
    for r in rules.iter().filter(|r| broken.contains(&r.id)) {
        crate::notify::bell(&state.app, &account.id, "Uma regra foi desligada", format!("A pasta de destino de \"{}\" não existe mais.", r.name));
    }

    let mut seen_uids = Vec::new();
    let mut star_uids = Vec::new();
    let mut copies: BTreeMap<i64, (FolderRow, Vec<u32>)> = BTreeMap::new();
    let mut moves: BTreeMap<i64, Vec<(i64, u32)>> = BTreeMap::new();
    let mut followups: Vec<(i64, String)> = Vec::new();
    let mut per_rule: BTreeMap<i64, (Summary, Vec<RuleHit>)> = BTreeMap::new();

    for (f, plan) in &items {
        for (rule_id, actions) in &plan.steps {
            if broken.contains(rule_id) {
                continue;
            }
            let rule = rules.iter().find(|r| r.id == *rule_id).expect("plano de uma regra carregada");
            let (summary, hits) = per_rule.entry(*rule_id).or_default();
            let mut to_folder = None;
            for a in actions {
                let index = rule.spec.actions.iter().position(|x| x == a).unwrap_or(0);
                match a {
                    Action::Move { .. } | Action::Archive | Action::Delete => {
                        let Some(target) = targets.get(&(*rule_id, index)) else { continue };
                        if target.id == f.folder_id {
                            continue;
                        }
                        moves.entry(target.id).or_default().push((f.folder_id, f.uid));
                        to_folder = Some(target.id);
                        match a {
                            Action::Move { .. } => {
                                summary.moved += 1;
                                summary.folder = Some(target.display_name.clone());
                            }
                            Action::Archive => summary.archived += 1,
                            _ => summary.deleted += 1,
                        }
                    }
                    Action::Copy { .. } => {
                        let Some(target) = targets.get(&(*rule_id, index)) else { continue };
                        copies.entry(target.id).or_insert_with(|| (target.clone(), Vec::new())).1.push(f.uid);
                        summary.copied += 1;
                        summary.folder = Some(target.display_name.clone());
                    }
                    Action::MarkRead if !f.seen => {
                        seen_uids.push(f.uid);
                        summary.read += 1;
                    }
                    Action::Star if !f.flagged => {
                        star_uids.push(f.uid);
                        summary.starred += 1;
                    }
                    Action::Followup { days } => {
                        if !f.flagged {
                            star_uids.push(f.uid);
                        }
                        followups.push((f.message_id, due_date(*days)));
                        summary.followup += 1;
                    }
                    _ => {}
                }
            }
            hits.push(RuleHit { message_id: f.message_id, from_folder_id: f.folder_id, to_folder_id: to_folder, seen_before: f.seen, flagged_before: f.flagged });
        }
    }
    per_rule.retain(|_, (s, _)| s.moved + s.archived + s.deleted + s.copied + s.read + s.starred + s.followup > 0);
    if per_rule.is_empty() {
        return Ok(done);
    }

    // Servidor: flags e marcadores na pasta de origem, antes de o mover tirar o e-mail dela.
    seen_uids.sort_unstable();
    seen_uids.dedup();
    star_uids.sort_unstable();
    star_uids.dedup();
    if !seen_uids.is_empty() || !star_uids.is_empty() || !copies.is_empty() {
        if let Err(e) = server_changes(state, account, folder, &seen_uids, &star_uids, &copies).await {
            tracing::warn!(kind = e.kind(), "flags ou marcadores das regras ficaram para depois");
        }
    }

    // Banco: prazos, movimentos e histórico.
    let (account_id, origin_s) = (account.id.clone(), origin.to_owned());
    let names: HashMap<i64, String> = rules.iter().map(|r| (r.id, r.name.clone())).collect();
    let batch = format!("regras-{}", store::now());
    for (summary, _) in per_rule.values() {
        done.total.moved += summary.moved;
        done.total.archived += summary.archived;
        done.total.deleted += summary.deleted;
    }
    for (_, hits) in per_rule.values() {
        done.messages.extend(hits.iter().map(|h| h.message_id));
    }
    let runs = per_rule;
    let (run_ids, moved_any) = state
        .db
        .call(move |c| {
            for (message_id, due) in &followups {
                store::set_followup(c, *message_id, store::FollowupChange::Set(Some(due)))?;
            }
            let now = store::now();
            let mut moved_any = false;
            for (to, items) in &moves {
                moved_any |= !store::create_pending_moves(c, &account_id, &batch, items, *to, now)?.is_empty();
            }
            let mut ids = Vec::new();
            for (rule_id, (summary, hits)) in &runs {
                let json = serde_json::to_string(summary).unwrap_or_else(|_| "{}".into());
                ids.push(store::record_rule_run(c, &account_id, *rule_id, &names[rule_id], &origin_s, &json, hits)?);
            }
            Ok((ids, moved_any))
        })
        .await?;
    if moved_any {
        let app = state.app.clone();
        tauri::async_runtime::spawn(async move { crate::moves::run_due(&app).await });
    }
    done.run_ids = run_ids;
    state.emit_accounts_changed();
    Ok(done)
}

/// Lida, estrela e marcadores no servidor, na pasta de origem, e as flags confirmadas no banco.
async fn server_changes(state: &AppState, account: &Account, folder: &FolderRow, seen: &[u32], star: &[u32], copies: &BTreeMap<i64, (FolderRow, Vec<u32>)>) -> Result<(), AppError> {
    let handle = state.handle(&account.id).await?;
    let uidvalidity = folder.uidvalidity.ok_or(AppError::MessageNotFound)?;
    let (mailbox, seen, star) = (folder.imap_name.clone(), seen.to_vec(), star.to_vec());
    let targets: Vec<(String, Vec<u32>)> = copies.values().map(|(f, uids)| (f.imap_name.clone(), uids.clone())).collect();
    let confirmed: HashMap<u32, MessageFlags> = imap_client::with_session(state, &handle, Lane::Sync, |s| {
        let (mailbox, seen, star, targets) = (mailbox.clone(), seen.clone(), star.clone(), targets.clone());
        async move {
            imap_client::select_checked(s, &mailbox, uidvalidity).await?;
            let mut flags = HashMap::new();
            if !seen.is_empty() {
                flags.extend(imap_client::store_flags(s, &seen, Some(true), None).await?);
            }
            if !star.is_empty() {
                flags.extend(imap_client::store_flags(s, &star, None, Some(true)).await?);
            }
            for (target, uids) in &targets {
                imap_client::copy_uids(s, uids, target).await?;
            }
            Ok(flags)
        }
        .boxed()
    })
    .await?;
    let folder_id = folder.id;
    state
        .db
        .call(move |c| {
            for (uid, flags) in &confirmed {
                if let Some(m) = store::message_by_uid(c, folder_id, *uid)? {
                    store::set_local_flags(c, m.id, *flags)?;
                }
            }
            Ok(())
        })
        .await
}

/// O que a prévia mostra de cada e-mail (CA2).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewItem {
    pub from: String,
    pub subject: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shadow {
    pub rule_name: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    /// Quantos e-mails foram olhados.
    pub checked: usize,
    pub count: usize,
    pub sample: Vec<PreviewItem>,
    /// Regra de cima que para as outras e pega alguns destes e-mails (CA9).
    pub shadowed: Option<Shadow>,
}

/// A pasta da prévia e de "Rodar agora": a pedida, ou a Caixa de entrada.
fn pick_folder(c: &rusqlite::Connection, account_id: &str, folder: Option<&str>) -> Result<FolderRow, AppError> {
    match folder {
        Some(key) => {
            let k = FolderKey::decode(key)?;
            if k.account != account_id {
                return Err(AppError::InvalidId);
            }
            store::folder_of_account(c, account_id, k.folder)
        }
        None => store::folder_by_role(c, account_id, Role::Inbox)?.ok_or(AppError::FolderNotFound),
    }
}

/// Prévia ao vivo do editor (CA2): roda no banco local, sem rede. `all` olha a pasta inteira (a
/// contagem de "Rodar agora"); senão, os 500 mais recentes.
pub async fn preview(state: &AppState, account_id: String, rule_id: Option<i64>, spec: RuleSpec, folder: Option<String>, all: bool) -> Result<Preview, AppError> {
    let today = Local::now().date_naive();
    state
        .db
        .call(move |c| {
            let account = store::require_account(c, &account_id)?;
            let target = pick_folder(c, &account_id, folder.as_deref())?;
            let facts = store::rule_facts(c, target.id, None, (!all).then_some(PREVIEW_WINDOW), today)?;
            let hits: Vec<&Facts> = facts.iter().filter(|f| rules::matches(&spec, f, &account.email)).collect();
            let above: Vec<Rule> = store::active_rules(c, &account_id)?.into_iter().take_while(|r| Some(r.id) != rule_id).collect();
            let shadowed = rules::shadowed_by(&above, &spec, &facts, &account.email).map(|(r, n)| Shadow { rule_name: r.name.clone(), count: n });
            Ok(Preview {
                checked: facts.len(),
                count: hits.len(),
                sample: hits
                    .iter()
                    .take(PREVIEW_SAMPLE)
                    .map(|f| PreviewItem { from: if f.from_name.is_empty() { f.from_address.clone() } else { f.from_name.clone() }, subject: f.subject.clone(), date: String::new() })
                    .collect(),
                shadowed,
            })
        })
        .await
}

/// "Rodar agora" (CA4): a regra nos e-mails que já estão na pasta. Devolve quantos foram afetados.
pub async fn run_now(state: &AppState, account_id: String, rule_id: i64, folder: Option<String>) -> Result<usize, AppError> {
    let today = Local::now().date_naive();
    let id = account_id.clone();
    let (account, target, rule, facts) = state
        .db
        .call(move |c| {
            let account = store::require_account(c, &id)?;
            let row = store::get_rule(c, &id, rule_id)?;
            let target = pick_folder(c, &id, folder.as_deref())?;
            if never_runs_in(target.role) {
                return Err(AppError::InvalidInput);
            }
            let facts = store::rule_facts(c, target.id, None, None, today)?;
            Ok((account, target, Rule { id: row.id, name: row.name, spec: row.spec }, facts))
        })
        .await?;
    if account.status != AccountStatus::Active {
        return Err(AppError::SignedOut);
    }
    let rules = vec![rule];
    let items: Vec<(Facts, Plan)> = facts.into_iter().map(|f| {
        let p = rules::plan(&rules, &f, &account.email);
        (f, p)
    }).filter(|(_, p)| !p.steps.is_empty()).collect();
    let done = execute(state, &account, &target, &rules, items, "manual").await?;
    tracing::info!(count = done.messages.len(), "regra rodada agora");
    Ok(done.messages.len())
}

/// Limpeza diária (CA8): regras com "mais antigo que" ou "não é o mais recente do remetente", uma vez
/// por dia, com o app desbloqueado. Também tira do histórico os lotes com mais de 30 dias (D8).
pub fn start_cleanup(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        state.wait_started().await;
        loop {
            cleanup_once(&state).await;
            tokio::time::sleep(CLEANUP_TICK).await;
        }
    });
}

async fn cleanup_once(state: &AppState) {
    let today = Local::now().date_naive();
    let day = today.format("%Y-%m-%d").to_string();
    let Ok(last) = state.db.call(|c| store::rules_cleanup_day(c)).await else { return };
    if last.as_deref() == Some(day.as_str()) {
        return;
    }
    let Ok(accounts) = state.db.call(|c| store::list_accounts(c)).await else { return };
    for account in accounts.into_iter().filter(|a| a.status == AccountStatus::Active) {
        let id = account.id.clone();
        let Ok(rules) = state.db.call(move |c| store::active_rules(c, &id)).await else { continue };
        for rule in rules.into_iter().filter(|r| r.spec.is_cleanup()) {
            let (id, spec) = (account.id.clone(), rule.spec.clone());
            let folders = state
                .db
                .call(move |c| {
                    let mut out = Vec::new();
                    if let Some(inbox) = store::folder_by_role(c, &id, Role::Inbox)? {
                        out.push(inbox);
                    }
                    for key in &spec.folders {
                        if let Ok(k) = FolderKey::decode(key) {
                            if let Some(f) = store::get_folder(c, k.folder)?.filter(|f| f.account_id == id && !never_runs_in(f.role)) {
                                out.push(f);
                            }
                        }
                    }
                    Ok(out)
                })
                .await
                .unwrap_or_default();
            let rules = vec![rule.clone()];
            for folder in folders {
                let folder_id = folder.id;
                let Ok(facts) = state.db.call(move |c| store::rule_facts(c, folder_id, None, None, today)).await else { continue };
                let items: Vec<(Facts, Plan)> = facts.into_iter().map(|f| {
                    let p = rules::plan(&rules, &f, &account.email);
                    (f, p)
                }).filter(|(_, p)| !p.steps.is_empty()).collect();
                match execute(state, &account, &folder, &rules, items, "cleanup").await {
                    Ok(done) if done.messages.len() > BELL_BATCH => {
                        crate::notify::bell(&state.app, &account.id, "Limpeza das regras", format!("\"{}\" organizou {} e-mails", rule.name, done.messages.len()));
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!(kind = e.kind(), "limpeza de uma regra ficou para depois"),
                }
            }
        }
    }
    let _ = state
        .db
        .call(move |c| {
            store::prune_rule_runs(c)?;
            store::set_rules_cleanup_day(c, &day)
        })
        .await;
    tracing::info!("limpeza diária das regras concluída");
}

/// Desfaz um lote do histórico (CA6): o que foi movido volta para a pasta de origem e lida e estrela
/// voltam ao que eram. Marcadores não são desfeitos. Devolve quantos e-mails voltaram.
pub async fn undo(state: &AppState, account_id: String, run_id: i64) -> Result<usize, AppError> {
    let id = account_id.clone();
    let (account, plan) = state
        .db
        .call(move |c| {
            let account = store::require_account(c, &id)?;
            let mut moves: BTreeMap<i64, Vec<(i64, u32)>> = BTreeMap::new();
            let mut copy_back: Vec<(FolderRow, FolderRow, u32)> = Vec::new();
            let mut flags: BTreeMap<i64, Vec<(u32, MessageFlags)>> = BTreeMap::new();
            let mut restored = 0;
            for hit in store::rule_run_hits(c, &id, run_id)? {
                let before = MessageFlags { seen: hit.seen_before, flagged: hit.flagged_before };
                // Ainda na origem (movimento não executado): voltar é cancelar o movimento.
                if let Some(uid) = store::uid_in_folder(c, hit.message_id, hit.from_folder_id)? {
                    if hit.to_folder_id.is_some() {
                        moves.entry(hit.from_folder_id).or_default().push((hit.from_folder_id, uid));
                    }
                    flags.entry(hit.from_folder_id).or_default().push((uid, before));
                    restored += 1;
                    continue;
                }
                let Some(to) = hit.to_folder_id else { continue };
                let Some(uid) = store::uid_in_folder(c, hit.message_id, to)? else { continue };
                let (Some(dest), Some(origin)) = (store::get_folder(c, to)?, store::get_folder(c, hit.from_folder_id)?) else { continue };
                // Sair de "Todos os e-mails" apagaria a mensagem no servidor: copiar de volta.
                if dest.role == Role::All {
                    copy_back.push((dest, origin, uid));
                } else {
                    moves.entry(hit.from_folder_id).or_default().push((to, uid));
                }
                flags.entry(to).or_default().push((uid, before));
                restored += 1;
            }
            store::mark_rule_run_undone(c, run_id)?;
            let batch = format!("desfazer-regra-{run_id}");
            let now = store::now();
            for (to, items) in &moves {
                store::create_pending_moves(c, &id, &batch, items, *to, now)?;
            }
            Ok((account, (restored, copy_back, flags)))
        })
        .await?;
    let (restored, copy_back, flags) = plan;
    let app = state.app.clone();
    tauri::async_runtime::spawn(async move { crate::moves::run_due(&app).await });
    if let Err(e) = restore_on_server(state, &account, copy_back, flags).await {
        tracing::warn!(kind = e.kind(), "parte do desfazer das regras ficou para depois");
    }
    state.emit_accounts_changed();
    tracing::info!(count = restored, "lote de regra desfeito");
    Ok(restored)
}

async fn restore_on_server(state: &AppState, account: &Account, copy_back: Vec<(FolderRow, FolderRow, u32)>, flags: BTreeMap<i64, Vec<(u32, MessageFlags)>>) -> Result<(), AppError> {
    if copy_back.is_empty() && flags.is_empty() {
        return Ok(());
    }
    let handle = state.handle(&account.id).await?;
    let folders: HashMap<i64, FolderRow> = {
        let ids: Vec<i64> = flags.keys().copied().collect();
        state.db.call(move |c| ids.iter().filter_map(|id| store::get_folder(c, *id).transpose()).map(|f| f.map(|f| (f.id, f))).collect()).await?
    };
    for (folder_id, items) in flags {
        let Some(folder) = folders.get(&folder_id) else { continue };
        let Some(uidvalidity) = folder.uidvalidity else { continue };
        let mailbox = folder.imap_name.clone();
        imap_client::with_session(state, &handle, Lane::Sync, |s| {
            let (mailbox, items) = (mailbox.clone(), items.clone());
            async move {
                imap_client::select_checked(s, &mailbox, uidvalidity).await?;
                for (uid, f) in &items {
                    imap_client::store_flags(s, &[*uid], Some(f.seen), Some(f.flagged)).await?;
                }
                Ok(())
            }
            .boxed()
        })
        .await?;
    }
    for (from, to, uid) in copy_back {
        let Some(uidvalidity) = from.uidvalidity else { continue };
        imap_client::with_session(state, &handle, Lane::Sync, |s| {
            let (mailbox, target) = (from.imap_name.clone(), to.imap_name.clone());
            async move {
                imap_client::select_checked(s, &mailbox, uidvalidity).await?;
                imap_client::copy_uids(s, &[uid], &target).await
            }
            .boxed()
        })
        .await?;
    }
    Ok(())
}

/// Sugestões para "Criar regra" a partir de uma mensagem (CA5).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleSuggestions {
    pub from_name: String,
    pub from_address: String,
    pub domain: String,
    pub list_id: Option<String>,
    pub subject: String,
    /// Pasta onde a mensagem está.
    pub folder_id: String,
}

pub fn suggestions(c: &rusqlite::Connection, account_id: &str, folder_id: i64, uid: u32) -> Result<RuleSuggestions, AppError> {
    let folder = store::folder_of_account(c, account_id, folder_id)?;
    let facts = store::rule_facts(c, folder.id, None, None, Local::now().date_naive())?;
    let f = facts.into_iter().find(|f| f.uid == uid).ok_or(AppError::MessageNotFound)?;
    let domain = f.from_address.rsplit_once('@').map(|(_, d)| d.to_lowercase()).unwrap_or_default();
    Ok(RuleSuggestions {
        from_name: f.from_name,
        from_address: f.from_address,
        domain,
        list_id: f.list_id.filter(|l| l != "-"),
        subject: f.subject,
        folder_id: FolderKey { account: account_id.to_owned(), folder: folder.id }.encode(),
    })
}
