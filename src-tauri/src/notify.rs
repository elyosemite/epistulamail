//! Notificações do Windows (US8, CA1–CA4). Os textos são montados aqui, a partir do banco, e
//! respeitam a privacidade escolhida ("só a conta" não mostra remetente nem assunto). Clicar na
//! notificação traz a janela de volta da bandeja e abre a conta e a mensagem (evento `open-target`).
//! Nunca vão para log: remetente, assunto ou trecho. Só tipo e quantidade.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{Datelike, Local, Timelike};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppError;
use crate::message::{FolderKey, MessageKey};
use crate::session::AppState;
use crate::store::{self, NewMail, NotifySettings};

/// Até quantos e-mails novos de uma vez viram notificações separadas (D4).
pub const MAX_SEPARATE: usize = 3;
const OFFLINE_AFTER: Duration = Duration::from_secs(3600);
const TICK: Duration = Duration::from_secs(30);

/// Para onde o clique leva.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub account_id: String,
    /// Pasta da mensagem; sem ela, a Caixa de entrada da conta.
    pub folder_id: Option<String>,
    /// Mensagem a abrir; sem ela, só a pasta.
    pub message_id: Option<String>,
    /// Abrir o login da conta ("Reconecte").
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub reconnect: bool,
}

/// Uma notificação pronta: título e até duas linhas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// Tipo, para o ícone no sino: mail, reply, summary, due, failure, reauth, offline, info.
    pub kind: &'static str,
    /// Remetente, para o avatar no sino (só com a privacidade "Remetente e assunto").
    pub sender: Option<String>,
    pub title: String,
    pub line1: String,
    pub line2: String,
    pub target: Option<Target>,
}

/// Estado em memória: preferências em cache (lidas pelo fechar da janela, que não pode esperar o
/// banco), Não perturbe da bandeja, o que chegou durante o silêncio e a saúde das contas.
#[derive(Default)]
pub struct Notifier {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    settings: NotifySettings,
    /// "Não perturbe por 1 hora" da bandeja.
    snooze_until: Option<Instant>,
    /// Contas que já tiveram a Caixa de entrada sincronizada nesta abertura do app.
    started: HashSet<String>,
    /// Durante o silêncio: e-mails novos e respostas por conta (nome).
    held: HashMap<String, (usize, usize)>,
    was_quiet: bool,
    was_snoozed: bool,
    offline_since: HashMap<String, Instant>,
    offline_notified: HashSet<String>,
    /// Último dia (AAAA-MM-DD) em que o aviso dos prazos saiu.
    due_sent_on: Option<String>,
}

impl Notifier {
    pub fn settings(&self) -> NotifySettings {
        self.inner.lock().map(|i| i.settings.clone()).unwrap_or_default()
    }

    pub fn set_settings(&self, settings: NotifySettings) {
        if let Ok(mut i) = self.inner.lock() {
            i.settings = settings;
        }
    }

    /// Liga e desliga o "Não perturbe por 1 hora" (bandeja e sino). Devolve se ficou ligado.
    pub fn toggle_snooze(&self) -> bool {
        let on = !self.snoozed();
        self.set_snooze(on)
    }

    pub fn set_snooze(&self, on: bool) -> bool {
        let Ok(mut i) = self.inner.lock() else { return false };
        i.snooze_until = on.then(|| Instant::now() + Duration::from_secs(3600));
        on
    }

    pub fn snoozed(&self) -> bool {
        self.inner.lock().map(|i| i.snooze_until.is_some_and(|u| u > Instant::now())).unwrap_or(false)
    }

    pub fn quiet_now(&self) -> bool {
        let Ok(i) = self.inner.lock() else { return false };
        i.snooze_until.is_some_and(|u| u > Instant::now()) || (i.settings.quiet && in_quiet_hours(Local::now().hour(), i.settings.quiet_from, i.settings.quiet_to))
    }
}

/// Entre `from` e `to` (horas cheias), atravessando a meia-noite quando `from > to`.
pub fn in_quiet_hours(hour: u32, from: u32, to: u32) -> bool {
    match from.cmp(&to) {
        std::cmp::Ordering::Less => hour >= from && hour < to,
        std::cmp::Ordering::Greater => hour >= from || hour < to,
        std::cmp::Ordering::Equal => false,
    }
}

fn sender(m: &NewMail) -> String {
    let name = m.from_name.trim();
    if name.is_empty() { m.from_address.trim().to_owned() } else { name.to_owned() }
}

/// Notificações de e-mails novos de uma conta (D4): até `MAX_SEPARATE`, uma por e-mail; acima, um
/// resumo. Respostas a e-mails seus ganham a própria notificação (mesmo com os novos desligados).
pub fn new_mail_notes(settings: &NotifySettings, account_id: &str, account_name: &str, mails: &[NewMail], replies: &HashSet<(i64, u32)>) -> Vec<Note> {
    let full = settings.privacy == "full";
    let key = |m: &NewMail| MessageKey { account: account_id.to_owned(), folder: m.folder_id, uid: m.uid }.encode();
    let folder = |m: &NewMail| FolderKey { account: account_id.to_owned(), folder: m.folder_id }.encode();
    let target = |m: &NewMail| Some(Target { account_id: account_id.to_owned(), folder_id: Some(folder(m)), message_id: Some(key(m)), reconnect: false });
    let is_reply = |m: &NewMail| replies.contains(&(m.folder_id, m.uid));
    let mut notes = Vec::new();
    let (reply_mails, others): (Vec<&NewMail>, Vec<&NewMail>) = mails.iter().partition(|m| is_reply(m));
    if settings.replies {
        for m in &reply_mails {
            notes.push(if full {
                Note { kind: "reply", sender: Some(sender(m)), title: format!("{} respondeu", sender(m)), line1: m.subject.clone(), line2: format!("{account_name} · resposta a um e-mail seu"), target: target(m) }
            } else {
                Note { kind: "reply", sender: None, title: format!("Nova resposta em {account_name}"), line1: "Alguém respondeu um e-mail seu.".into(), line2: String::new(), target: target(m) }
            });
        }
    }
    let rest: Vec<&NewMail> = if settings.replies { others } else { mails.iter().collect() };
    if !settings.new_mail || rest.is_empty() {
        return notes;
    }
    if rest.len() > MAX_SEPARATE {
        let names: Vec<String> = rest.iter().map(|m| sender(m)).fold(Vec::new(), |mut acc, n| {
            if !acc.contains(&n) {
                acc.push(n);
            }
            acc
        });
        let line1 = if !full {
            "Abra o Epistula para ler.".to_owned()
        } else if names.len() > 3 {
            format!("{} e mais {}", names[..3].join(", "), names.len() - 3)
        } else {
            join_names(&names)
        };
        notes.push(Note { kind: "summary", sender: None, title: format!("{} e-mails novos em {account_name}", rest.len()), line1, line2: String::new(), target: Some(newest_target(account_id, &rest)) });
        return notes;
    }
    for m in rest {
        notes.push(if full {
            Note {
                kind: "mail",
                sender: Some(sender(m)),
                title: sender(m),
                line1: m.subject.clone(),
                line2: match m.snippet.as_deref().filter(|s| !s.is_empty()) {
                    Some(snippet) => format!("{snippet} · {account_name}"),
                    None => account_name.to_owned(),
                },
                target: target(m),
            }
        } else {
            Note { kind: "mail", sender: None, title: format!("Novo e-mail em {account_name}"), line1: "Abra o Epistula para ler.".into(), line2: String::new(), target: target(m) }
        });
    }
    notes
}

fn join_names(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} e {last}", init.join(", ")),
    }
}

/// Mostra a notificação no Windows. O clique traz a janela e manda o destino ao frontend.
pub fn show(app: &AppHandle, note: Note) {
    #[cfg(windows)]
    {
        use tauri_winrt_notification::Toast;
        // Em desenvolvimento o app não tem atalho no Menu Iniciar (AppUserModelID): o Windows só
        // mostra a notificação com um id conhecido.
        let id = if cfg!(debug_assertions) { Toast::POWERSHELL_APP_ID.to_owned() } else { app.config().identifier.clone() };
        let handle = app.clone();
        let target = note.target.clone();
        let result = Toast::new(&id)
            .title(&note.title)
            .text1(&note.line1)
            .text2(&note.line2)
            .on_activated(move |_| {
                bring_to_front(&handle);
                if let Some(t) = &target {
                    let _ = handle.emit("open-target", t.clone());
                }
                Ok(())
            })
            .show();
        if let Err(e) = result {
            tracing::warn!(error = %e, "notificação não pôde ser mostrada");
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (app, note);
    }
}

/// Guarda no histórico do sino e avisa o frontend para atualizar o contador (CA7).
fn record(app: &AppHandle, notes: &[Note]) {
    let items: Vec<store::NewNotice> = notes
        .iter()
        .map(|n| store::NewNotice {
            account_id: n.target.as_ref().map(|t| t.account_id.clone()),
            kind: n.kind.to_owned(),
            title: n.title.clone(),
            line1: n.line1.clone(),
            line2: n.line2.clone(),
            sender: n.sender.clone(),
            target: n.target.as_ref().and_then(|t| serde_json::to_value(t).ok()),
        })
        .collect();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let at = store::now();
        if state.db.call(move |c| store::add_notices(c, &items, at)).await.is_ok() {
            let _ = app.emit("notices-updated", ());
        }
    });
}

/// Mostra a janela escondida na bandeja e a põe em foco.
pub fn bring_to_front(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn window_in_focus(app: &AppHandle) -> bool {
    app.get_webview_window("main").is_some_and(|w| w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false))
}

fn notifier(app: &AppHandle) -> Option<tauri::State<'_, Notifier>> {
    app.try_state::<Notifier>()
}

fn account_on(settings: &NotifySettings, account_id: &str) -> bool {
    !settings.accounts_off.iter().any(|a| a == account_id)
}

/// Depois de sincronizar a Caixa de entrada: notifica os não lidos que chegaram (CA1).
pub async fn after_inbox_sync(app: &AppHandle, account_id: &str, folder_id: i64, new_ids: Vec<i64>, rules: crate::rule_runner::NewMailOutcome) {
    // Regras (US12, CA3): o que saiu da Caixa de entrada não avisa; "Notificar sempre" avisa já.
    let always: Vec<i64> = new_ids.iter().copied().filter(|i| rules.always.contains(i)).collect();
    let new_ids: Vec<i64> = new_ids.into_iter().filter(|i| !rules.silent.contains(i) && !rules.always.contains(i)).collect();
    if !always.is_empty() {
        urgent(app, account_id, folder_id, always).await;
    }
    let Some(n) = notifier(app) else { return };
    let settings = n.settings();
    let first = n.inner.lock().map(|mut i| i.started.insert(account_id.to_owned())).unwrap_or(false);
    if new_ids.is_empty() || !account_on(&settings, account_id) || (!settings.new_mail && !settings.replies) {
        return;
    }
    let state = app.state::<AppState>();
    let id = account_id.to_owned();
    let loaded = state
        .db
        .call(move |c| {
            let account = store::require_account(c, &id)?;
            let mails: Vec<NewMail> = store::new_mail(c, folder_id, &new_ids)?.into_iter().filter(|m| !m.seen).collect();
            let replies = store::replies_to_me(c, &id, &account.email, &new_ids)?;
            let reply_keys = store::new_mail(c, folder_id, &replies)?.into_iter().map(|m| (m.folder_id, m.uid)).collect::<HashSet<_>>();
            Ok((account, mails, reply_keys))
        })
        .await;
    let Ok((account, mails, replies)) = loaded else { return };
    if mails.is_empty() {
        return;
    }
    // Primeira sincronização desta abertura: só um resumo do que chegou com o app fechado.
    let notes = if first {
        vec![closed_summary(account_id, &account.display_name, &mails)]
    } else {
        new_mail_notes(&settings, account_id, &account.display_name, &mails, &replies)
    };
    deliver(app, &account.display_name, notes, mails.len(), replies.len());
}

/// O destino de um aviso sobre vários e-mails: a pasta e o mais recente deles, aberto e marcado na lista.
fn newest_target(account_id: &str, mails: &[&NewMail]) -> Target {
    let newest = mails.iter().max_by_key(|m| m.uid);
    Target {
        account_id: account_id.to_owned(),
        folder_id: newest.map(|m| FolderKey { account: account_id.to_owned(), folder: m.folder_id }.encode()),
        message_id: newest.map(|m| MessageKey { account: account_id.to_owned(), folder: m.folder_id, uid: m.uid }.encode()),
        reconnect: false,
    }
}

/// O que chegou enquanto o app estava fechado, num aviso só (sem remetentes).
pub fn closed_summary(account_id: &str, account_name: &str, mails: &[NewMail]) -> Note {
    let count = mails.len();
    let what = if count == 1 { "1 e-mail novo".to_owned() } else { format!("{count} e-mails novos") };
    Note {
        kind: "summary",
        sender: None,
        title: format!("{what} em {account_name}"),
        line1: "Chegaram enquanto o Epistula estava fechado.".into(),
        line2: String::new(),
        target: Some(newest_target(account_id, &mails.iter().collect::<Vec<_>>())),
    }
}

/// Entrega ou guarda para o resumo do fim do silêncio.
fn deliver(app: &AppHandle, account_name: &str, notes: Vec<Note>, mails: usize, replies: usize) {
    let Some(n) = notifier(app) else { return };
    if notes.is_empty() {
        return;
    }
    record(app, &notes);
    if n.quiet_now() {
        if let Ok(mut i) = n.inner.lock() {
            let entry = i.held.entry(account_name.to_owned()).or_default();
            entry.0 += mails.saturating_sub(replies);
            entry.1 += replies;
        }
        tracing::info!(count = notes.len(), "notificações guardadas para depois do silêncio");
        return;
    }
    tracing::info!(count = notes.len(), "notificações mostradas");
    for note in notes {
        show(app, note);
    }
}

/// Aviso só no sino, sem notificação do Windows (regras, US12).
pub fn bell(app: &AppHandle, account_id: &str, title: &str, line1: String) {
    let note = Note {
        kind: "info",
        sender: None,
        title: title.to_owned(),
        line1,
        line2: String::new(),
        target: Some(Target { account_id: account_id.to_owned(), folder_id: None, message_id: None, reconnect: false }),
    };
    record(app, std::slice::from_ref(&note));
}

/// "Notificar sempre" (US12, D6): mostra mesmo no Não perturbe, no horário de silêncio e com os
/// e-mails novos desligados.
async fn urgent(app: &AppHandle, account_id: &str, folder_id: i64, ids: Vec<i64>) {
    let Some(n) = notifier(app) else { return };
    let settings = n.settings();
    let state = app.state::<AppState>();
    let id = account_id.to_owned();
    let loaded = state.db.call(move |c| Ok((store::require_account(c, &id)?, store::new_mail(c, folder_id, &ids)?))).await;
    let Ok((account, mails)) = loaded else { return };
    let notes = new_mail_notes(&settings, account_id, &account.display_name, &mails, &HashSet::new());
    if notes.is_empty() {
        return;
    }
    record(app, &notes);
    tracing::info!(count = notes.len(), "notificações de regra mostradas mesmo no silêncio");
    for note in notes {
        show(app, note);
    }
}

/// Falha que o usuário precisa saber com a janela fora de foco (envio e mover, CA2).
pub fn failure(app: &AppHandle, account_id: &str, title: String, line1: String, line2: String) {
    let Some(n) = notifier(app) else { return };
    let settings = n.settings();
    if !settings.failures || !account_on(&settings, account_id) || window_in_focus(app) {
        return;
    }
    let note = Note { kind: "failure", sender: None, title, line1, line2, target: Some(Target { account_id: account_id.to_owned(), folder_id: None, message_id: None, reconnect: false }) };
    record(app, std::slice::from_ref(&note));
    if !n.quiet_now() {
        show(app, note);
    }
}

/// A conta perdeu o acesso (CA3). Uma vez por perda: o status só muda uma vez.
pub fn reauth(app: &AppHandle, account_id: &str, account_name: &str, email: &str) {
    let Some(n) = notifier(app) else { return };
    let settings = n.settings();
    if !settings.reauth || !account_on(&settings, account_id) {
        return;
    }
    let note = Note {
        kind: "reauth",
        sender: None,
        title: format!("Reconecte {account_name}"),
        line1: format!("O acesso a {email} terminou."),
        line2: "Entre de novo para continuar recebendo e-mails.".into(),
        target: Some(Target { account_id: account_id.to_owned(), folder_id: None, message_id: None, reconnect: true }),
    };
    record(app, std::slice::from_ref(&note));
    show(app, note);
}

/// Resultado de uma sincronização: guarda desde quando a conta está sem conexão (CA3).
pub fn connection(app: &AppHandle, account_id: &str, online: bool) {
    let Some(n) = notifier(app) else { return };
    if let Ok(mut i) = n.inner.lock() {
        if online {
            i.offline_since.remove(account_id);
            i.offline_notified.remove(account_id);
        } else {
            i.offline_since.entry(account_id.to_owned()).or_insert_with(Instant::now);
        }
    };
}

/// Primeira vez que a janela fecha: avisa que o app continua na bandeja (CA5).
pub fn tray_hint(app: &AppHandle) {
    show(
        app,
        Note {
            kind: "info",
            sender: None,
            title: "O Epistula continua na bandeja".into(),
            line1: "Para avisar sobre e-mails novos.".into(),
            line2: "Para sair, use Sair no ícone da bandeja.".into(),
            target: None,
        },
    );
}

/// Relógio das notificações: fim do silêncio, prazos do dia e contas sem conexão.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        state.wait_started().await;
        if let Ok(settings) = state.db.call(|c| store::notify_settings(c)).await {
            if let Some(n) = notifier(&app) {
                n.set_settings(settings);
            }
        }
        loop {
            tick(&app).await;
            tokio::time::sleep(TICK).await;
        }
    });
}

async fn tick(app: &AppHandle) {
    let Some(n) = notifier(app) else { return };
    let quiet = n.quiet_now();
    let settings = n.settings();
    // Fim do silêncio: um resumo do que chegou.
    let snoozed = n.snoozed();
    let (changed, held) = n
        .inner
        .lock()
        .map(|mut i| {
            let changed = i.was_quiet != quiet || i.was_snoozed != snoozed;
            let ended = i.was_quiet && !quiet;
            (i.was_quiet, i.was_snoozed) = (quiet, snoozed);
            if !snoozed {
                i.snooze_until = None;
            }
            (changed, ended.then(|| std::mem::take(&mut i.held)))
        })
        .unwrap_or((false, None));
    // O sino troca de ícone e a bandeja de texto quando o silêncio começa ou acaba.
    if changed {
        crate::tray::sync_snooze(app);
        let _ = app.emit("notices-updated", ());
    }
    if let Some(held) = held.filter(|h| !h.is_empty()) {
        let mut parts: Vec<String> = held.iter().filter(|(_, (m, _))| *m > 0).map(|(a, (m, _))| format!("{m} {} em {a}", if *m == 1 { "e-mail novo" } else { "e-mails novos" })).collect();
        let replies: usize = held.values().map(|(_, r)| r).sum();
        if replies > 0 {
            parts.push(format!("{replies} {}", if replies == 1 { "resposta a um e-mail seu" } else { "respostas a e-mails seus" }));
        }
        show(app, Note { kind: "summary", sender: None, title: "Enquanto você estava em Não perturbe".into(), line1: join_names(&parts), line2: String::new(), target: None });
    }
    if quiet {
        return;
    }
    // Prazos do dia, uma vez por dia a partir da hora escolhida.
    let now = Local::now();
    let today = format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day());
    let due_pending = settings.due && now.hour() >= settings.due_hour && n.inner.lock().map(|i| i.due_sent_on.as_deref() != Some(today.as_str())).unwrap_or(false);
    if due_pending {
        if let Ok(mut i) = n.inner.lock() {
            i.due_sent_on = Some(today.clone());
        }
        due_today(app, &settings, today).await;
    }
    // Sem conexão há mais de 1 hora.
    if settings.offline {
        let late: Vec<String> = n
            .inner
            .lock()
            .map(|mut i| {
                let late: Vec<String> = i.offline_since.iter().filter(|(_, t)| t.elapsed() >= OFFLINE_AFTER).map(|(a, _)| a.clone()).filter(|a| !i.offline_notified.contains(a)).collect();
                i.offline_notified.extend(late.iter().cloned());
                late
            })
            .unwrap_or_default();
        if !late.is_empty() {
            let note = Note { kind: "offline", sender: None, title: "Sem conexão há 1 hora".into(), line1: "Os e-mails novos aparecem quando a conexão voltar.".into(), line2: String::new(), target: None };
            record(app, std::slice::from_ref(&note));
            show(app, note);
        }
    }
}

async fn due_today(app: &AppHandle, settings: &NotifySettings, today: String) {
    let state = app.state::<AppState>();
    let accounts = match state.db.call(|c| store::list_accounts(c)).await {
        Ok(a) => a,
        Err(_) => return,
    };
    for account in accounts.into_iter().filter(|a| a.status == store::AccountStatus::Active && account_on(settings, &a.id)) {
        let (id, day) = (account.id.clone(), today.clone());
        let found = state
            .db
            .call(move |c| {
                let dates = store::FlagDates { today: day.clone(), week_end: day };
                let counts = store::flagged_counts(c, &id, &dates)?;
                let first = store::flagged(c, &id, store::FlagBucket::Today, &dates)?.into_iter().next().or(store::flagged(c, &id, store::FlagBucket::Overdue, &dates)?.into_iter().next());
                Ok::<_, AppError>((counts, first))
            })
            .await;
        let Ok((counts, first)) = found else { continue };
        let total = counts.today + counts.overdue;
        if total == 0 {
            continue;
        }
        let title = if total == 1 { "1 sinalizada vence hoje".to_owned() } else { format!("{total} sinalizadas vencem hoje") };
        let mut line1 = match (&first, settings.privacy == "full") {
            (Some(hit), true) if total > 1 => format!("{} e mais {}.", hit.message.subject, total - 1),
            (Some(hit), true) => hit.message.subject.clone(),
            _ => "Abra o Epistula para ver.".into(),
        };
        if counts.overdue > 0 {
            line1.push_str(&format!(" {} {} atrasada{}.", counts.overdue, if counts.overdue == 1 { "está" } else { "estão" }, if counts.overdue == 1 { "" } else { "s" }));
        }
        let target = first.map(|hit| Target {
            account_id: account.id.clone(),
            folder_id: Some(FolderKey { account: account.id.clone(), folder: hit.folder_id }.encode()),
            message_id: Some(MessageKey { account: account.id.clone(), folder: hit.folder_id, uid: hit.message.uid }.encode()),
            reconnect: false,
        });
        tracing::info!(count = total, "aviso dos prazos");
        let note = Note { kind: "due", sender: None, title, line1, line2: account.display_name.clone(), target };
        record(app, std::slice::from_ref(&note));
        show(app, note);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mail(uid: u32, name: &str, subject: &str) -> NewMail {
        NewMail { folder_id: 1, uid, from_name: name.into(), from_address: format!("{uid}@x.com"), subject: subject.into(), snippet: Some("Oi.".into()), seen: false }
    }

    #[test]
    fn closed_summary_has_no_senders() {
        let twelve: Vec<_> = (1..=12).map(|i| mail(i, "Marina", "s")).collect();
        let n = closed_summary("acc", "Trabalho", &twelve);
        let target = n.target.as_ref().unwrap();
        assert_eq!(target.message_id, Some(MessageKey { account: "acc".into(), folder: 1, uid: 12 }.encode()), "abre a mais recente");
        assert!(target.folder_id.is_some());
        assert_eq!(n.title, "12 e-mails novos em Trabalho");
        assert_eq!(closed_summary("acc", "Pessoal", &twelve[..1]).title, "1 e-mail novo em Pessoal");
    }

    #[test]
    fn quiet_hours_cross_midnight() {
        assert!(in_quiet_hours(23, 22, 7) && in_quiet_hours(3, 22, 7));
        assert!(!in_quiet_hours(7, 22, 7) && !in_quiet_hours(12, 22, 7));
        assert!(in_quiet_hours(13, 12, 14) && !in_quiet_hours(14, 12, 14));
        assert!(!in_quiet_hours(5, 6, 6));
    }

    #[test]
    fn few_mails_are_separate_and_many_become_a_summary() {
        let s = NotifySettings { privacy: "full".into(), ..Default::default() };
        let three = [mail(1, "Marina Couto", "Contrato"), mail(2, "Rafael Dias", "Q1"), mail(3, "", "Extrato")];
        let notes = new_mail_notes(&s, "acc", "Trabalho", &three, &HashSet::new());
        assert_eq!(notes.len(), 3);
        assert_eq!((notes[0].title.as_str(), notes[0].line1.as_str(), notes[0].line2.as_str()), ("Marina Couto", "Contrato", "Oi. · Trabalho"));
        assert_eq!(notes[2].title, "3@x.com", "sem nome, o endereço");
        assert!(notes[0].target.as_ref().unwrap().message_id.is_some());

        let five: Vec<_> = (1..=5).map(|i| mail(i, &format!("P{i}"), "s")).collect();
        let notes = new_mail_notes(&s, "acc", "Trabalho", &five, &HashSet::new());
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "5 e-mails novos em Trabalho");
        assert_eq!(notes[0].line1, "P1, P2, P3 e mais 2");
        assert_eq!(notes[0].target.as_ref().unwrap().message_id, Some(MessageKey { account: "acc".into(), folder: 1, uid: 5 }.encode()), "o resumo abre a mais recente");
    }

    #[test]
    fn replies_have_their_own_note_even_with_new_mail_off() {
        let s = NotifySettings { new_mail: false, privacy: "full".into(), ..Default::default() };
        let mails = [mail(1, "Paulo Henrique", "Re: Contrato"), mail(2, "Banco", "Extrato")];
        let replies: HashSet<_> = [(1, 1)].into();
        let notes = new_mail_notes(&s, "acc", "Trabalho", &mails, &replies);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Paulo Henrique respondeu");
        assert_eq!(notes[0].line2, "Trabalho · resposta a um e-mail seu");
    }

    #[test]
    fn notifications_show_only_the_account_by_default() {
        // US10, D5: o Windows guarda o texto das notificações sem cifra.
        assert_eq!(NotifySettings::default().privacy, "account");
    }

    #[test]
    fn account_only_privacy_hides_sender_and_subject() {
        let s = NotifySettings { privacy: "account".into(), ..Default::default() };
        let mails = [mail(1, "Marina Couto", "Contrato secreto")];
        let notes = new_mail_notes(&s, "acc", "Trabalho", &mails, &[(1, 1)].into());
        let text = format!("{notes:?}");
        assert!(!text.contains("Marina") && !text.contains("secreto"), "{text}");
        assert_eq!(notes[0].title, "Nova resposta em Trabalho");
        let notes = new_mail_notes(&s, "acc", "Trabalho", &mails, &HashSet::new());
        assert_eq!(notes[0].title, "Novo e-mail em Trabalho");
    }
}
