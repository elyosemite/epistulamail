//! Regras (US12): o que é uma regra, quando ela casa com um e-mail e o que fazer. Tudo aqui é puro
//! (sem banco e sem rede) e testado; quem executa é `rule_runner.rs`. Comparações de texto ignoram
//! maiúsculas e acentos; vários valores separados por vírgula valem como "qualquer um".

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::message::Person;

/// Versão do formato guardado em `rule.spec`.
pub const SPEC_VERSION: u8 = 1;
const MAX_CONDITIONS: usize = 20;
const MAX_ACTIONS: usize = 10;
const MAX_VALUE_CHARS: usize = 500;
pub const MAX_NAME_CHARS: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    From,
    FromDomain,
    To,
    Cc,
    ToOrCc,
    OnlyToMe,
    Subject,
    Snippet,
    List,
    HasFiles,
    SizeOverKb,
    Seen,
    Flagged,
    /// Limpeza (CA8): só na limpeza diária e em "Rodar agora".
    OlderThanDays,
    NotNewestFromSender,
}

impl Field {
    fn is_text(self) -> bool {
        matches!(self, Field::From | Field::FromDomain | Field::To | Field::Cc | Field::ToOrCc | Field::Subject | Field::Snippet)
    }

    fn is_number(self) -> bool {
        matches!(self, Field::SizeOverKb | Field::OlderThanDays)
    }

    /// Condições de limpeza: dependem do tempo ou de outros e-mails, não do e-mail que chega.
    pub fn is_cleanup(self) -> bool {
        matches!(self, Field::OlderThanDays | Field::NotNewestFromSender)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Contains,
    NotContains,
    Is,
    StartsWith,
    EndsWith,
    /// Campos de sim ou não (lista, anexos, lida, estrela, só para mim).
    Yes,
    No,
    /// Campos numéricos (tamanho, idade).
    Over,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    pub field: Field,
    pub op: Op,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// Para uma pasta da conta (`FolderKey` codificado).
    Move { folder: String },
    Archive,
    Delete,
    /// Aplicar marcador: copia para a pasta.
    Copy { folder: String },
    MarkRead,
    Star,
    /// Estrela com prazo daqui a `days` dias (0 = hoje).
    Followup { days: u8 },
    Silence,
    NotifyAlways,
}

impl Action {
    /// Ações que tiram o e-mail da pasta: só uma por regra.
    pub fn leaves_folder(&self) -> bool {
        matches!(self, Action::Move { .. } | Action::Archive | Action::Delete)
    }
}

/// O que fica em `rule.spec`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleSpec {
    #[serde(default = "spec_version")]
    pub v: u8,
    /// Pastas onde a regra roda ao sincronizar, além da Caixa de entrada (D2); `FolderKey` codificado.
    #[serde(default)]
    pub folders: Vec<String>,
    /// "Todas" (verdadeiro) ou "qualquer uma" das condições.
    pub match_all: bool,
    pub conditions: Vec<Condition>,
    #[serde(default)]
    pub exceptions: Vec<Condition>,
    pub actions: Vec<Action>,
    #[serde(default)]
    pub stop: bool,
}

fn spec_version() -> u8 {
    SPEC_VERSION
}

impl RuleSpec {
    /// Confere limites e combinações (CA2). Sem condição vale para todos os e-mails: quem chama
    /// precisa ter pedido confirmação.
    pub fn validate(&self) -> Result<(), AppError> {
        let too_many = self.conditions.len() + self.exceptions.len() > MAX_CONDITIONS || self.actions.is_empty() || self.actions.len() > MAX_ACTIONS;
        if self.v != SPEC_VERSION || too_many {
            return Err(AppError::InvalidInput);
        }
        if self.actions.iter().filter(|a| a.leaves_folder()).count() > 1 {
            return Err(AppError::InvalidInput);
        }
        for c in self.conditions.iter().chain(&self.exceptions) {
            let op_ok = if c.field.is_text() {
                matches!(c.op, Op::Contains | Op::NotContains | Op::Is | Op::StartsWith | Op::EndsWith) && !values(&c.value).is_empty()
            } else if c.field.is_number() {
                c.op == Op::Over && c.value.trim().parse::<u32>().is_ok_and(|n| n > 0)
            } else {
                matches!(c.op, Op::Yes | Op::No)
            };
            if !op_ok || c.value.chars().count() > MAX_VALUE_CHARS {
                return Err(AppError::InvalidInput);
            }
        }
        Ok(())
    }

    /// Regra de limpeza: roda uma vez por dia e em "Rodar agora", nunca ao sincronizar.
    pub fn is_cleanup(&self) -> bool {
        self.conditions.iter().any(|c| c.field.is_cleanup())
    }
}

/// O que se sabe de um e-mail ao aplicar as regras (cabeçalho e prévia).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    pub message_id: i64,
    pub folder_id: i64,
    pub uid: u32,
    pub from_name: String,
    pub from_address: String,
    pub to: Vec<Person>,
    pub cc: Vec<Person>,
    pub subject: String,
    pub snippet: Option<String>,
    pub list_id: Option<String>,
    pub has_files: bool,
    pub size: u32,
    pub seen: bool,
    pub flagged: bool,
    /// Dias desde a data do e-mail.
    pub age_days: u32,
    /// É o mais recente deste remetente na pasta.
    pub newest_from_sender: bool,
}

/// Minúsculas e sem acentos, para comparar textos como a pessoa espera.
pub fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

/// Valores de uma condição: separados por vírgula, sem os vazios.
pub fn values(value: &str) -> Vec<String> {
    value.split(',').map(|v| fold(v.trim())).filter(|v| !v.is_empty()).collect()
}

fn text_matches(op: Op, haystacks: &[String], wanted: &[String]) -> bool {
    let hit = |h: &String, w: &String| match op {
        Op::Contains | Op::NotContains => h.contains(w.as_str()),
        Op::Is => h == w,
        Op::StartsWith => h.starts_with(w.as_str()),
        Op::EndsWith => h.ends_with(w.as_str()),
        _ => false,
    };
    let any = haystacks.iter().any(|h| wanted.iter().any(|w| hit(h, w)));
    if op == Op::NotContains {
        !any
    } else {
        any
    }
}

fn person_texts(people: &[&Person]) -> Vec<String> {
    people.iter().flat_map(|p| [fold(&p.address), fold(&p.name)]).filter(|t| !t.is_empty()).collect()
}

fn domain(address: &str) -> String {
    fold(address.rsplit_once('@').map_or("", |(_, d)| d))
}

pub fn condition_matches(c: &Condition, f: &Facts, me: &str) -> bool {
    let yes = |b: bool| if c.op == Op::No { !b } else { b };
    let number = || c.value.trim().parse::<u32>().unwrap_or(u32::MAX);
    let wanted = values(&c.value);
    let from = Person { name: f.from_name.clone(), address: f.from_address.clone() };
    match c.field {
        Field::From => text_matches(c.op, &person_texts(&[&from]), &wanted),
        Field::FromDomain => {
            // "termina com" é o natural para domínios; "é" casa também os subdomínios.
            let d = domain(&f.from_address);
            let subdomain = |w: &String| d.ends_with(&format!(".{w}"));
            match c.op {
                Op::Is => wanted.iter().any(|w| &d == w || subdomain(w)),
                _ => text_matches(c.op, &[d], &wanted),
            }
        }
        Field::To => text_matches(c.op, &person_texts(&f.to.iter().collect::<Vec<_>>()), &wanted),
        Field::Cc => text_matches(c.op, &person_texts(&f.cc.iter().collect::<Vec<_>>()), &wanted),
        Field::ToOrCc => text_matches(c.op, &person_texts(&f.to.iter().chain(&f.cc).collect::<Vec<_>>()), &wanted),
        Field::Subject => text_matches(c.op, &[fold(&f.subject)], &wanted),
        Field::Snippet => text_matches(c.op, &[fold(f.snippet.as_deref().unwrap_or_default())], &wanted),
        Field::OnlyToMe => {
            let me = fold(me);
            yes(f.cc.is_empty() && f.to.len() == 1 && fold(&f.to[0].address) == me)
        }
        Field::List => yes(f.list_id.is_some()),
        Field::HasFiles => yes(f.has_files),
        Field::Seen => yes(f.seen),
        Field::Flagged => yes(f.flagged),
        Field::SizeOverKb => f.size / 1024 > number(),
        Field::OlderThanDays => f.age_days > number(),
        Field::NotNewestFromSender => yes(!f.newest_from_sender),
    }
}

/// A regra casa com o e-mail? Sem condição, casa com todos.
pub fn matches(spec: &RuleSpec, f: &Facts, me: &str) -> bool {
    let check = |c: &Condition| condition_matches(c, f, me);
    let ok = spec.conditions.is_empty() || if spec.match_all { spec.conditions.iter().all(check) } else { spec.conditions.iter().any(check) };
    ok && !spec.exceptions.iter().any(check)
}

/// Uma regra pronta para rodar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub id: i64,
    pub name: String,
    pub spec: RuleSpec,
}

/// Como o e-mail deve ser notificado depois das regras.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Notify {
    #[default]
    Normal,
    Silent,
    Always,
}

/// O que as regras decidiram para um e-mail: as ações de cada regra que casou, na ordem.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plan {
    pub steps: Vec<(i64, Vec<Action>)>,
    pub notify: Notify,
}

/// Passa o e-mail pelas regras, em ordem. A primeira ação que tira o e-mail da pasta vale; as
/// seguintes são ignoradas (CA9). "Parar" encerra.
pub fn plan(rules: &[Rule], f: &Facts, me: &str) -> Plan {
    let mut out = Plan::default();
    let mut left = false;
    for rule in rules {
        if !matches(&rule.spec, f, me) {
            continue;
        }
        let mut actions = Vec::new();
        for a in &rule.spec.actions {
            if a.leaves_folder() {
                if left {
                    continue;
                }
                left = true;
            }
            match a {
                Action::NotifyAlways => out.notify = Notify::Always,
                Action::Silence if out.notify != Notify::Always => out.notify = Notify::Silent,
                _ => {}
            }
            actions.push(a.clone());
        }
        if !actions.is_empty() {
            out.steps.push((rule.id, actions));
        }
        if rule.spec.stop {
            break;
        }
    }
    // O que saiu da Caixa de entrada não vira "e-mail novo", salvo "Notificar sempre" (CA3).
    if left && out.notify == Notify::Normal {
        out.notify = Notify::Silent;
    }
    out
}

/// Regras de cima que param as seguintes e pegam algum destes e-mails (CA9: regra encoberta).
pub fn shadowed_by<'a>(rules: &'a [Rule], spec: &RuleSpec, facts: &[Facts], me: &str) -> Option<(&'a Rule, usize)> {
    rules.iter().filter(|r| r.spec.stop).find_map(|r| {
        let n = facts.iter().filter(|f| matches(spec, f, me) && matches(&r.spec, f, me)).count();
        (n > 0).then_some((r, n))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> Facts {
        Facts {
            message_id: 1,
            folder_id: 1,
            uid: 1,
            from_name: "Loja Ação".into(),
            from_address: "ofertas@news.loja.com.br".into(),
            to: vec![Person { name: "Eu".into(), address: "eu@exemplo.com".into() }],
            cc: vec![],
            subject: "Sua FATURA de outubro".into(),
            snippet: Some("Promoção só hoje".into()),
            list_id: Some("ofertas.loja.com.br".into()),
            has_files: true,
            size: 300 * 1024,
            seen: false,
            flagged: false,
            age_days: 20,
            newest_from_sender: false,
        }
    }

    fn cond(field: Field, op: Op, value: &str) -> Condition {
        Condition { field, op, value: value.into() }
    }

    fn spec(conditions: Vec<Condition>, actions: Vec<Action>) -> RuleSpec {
        RuleSpec { v: SPEC_VERSION, folders: vec![], match_all: true, conditions, exceptions: vec![], actions, stop: false }
    }

    fn rule(id: i64, s: RuleSpec) -> Rule {
        Rule { id, name: format!("r{id}"), spec: s }
    }

    const ME: &str = "eu@exemplo.com";

    #[test]
    fn text_ignores_case_and_accents_and_accepts_several_values() {
        let f = facts();
        assert!(condition_matches(&cond(Field::Subject, Op::Contains, "fatura"), &f, ME));
        assert!(condition_matches(&cond(Field::From, Op::Contains, "acao"), &f, ME), "sem acento casa com acento");
        assert!(condition_matches(&cond(Field::Snippet, Op::Contains, "PROMOCAO"), &f, ME));
        assert!(condition_matches(&cond(Field::Subject, Op::Contains, "boleto, fatura, nota fiscal"), &f, ME), "vírgula é qualquer um");
        assert!(!condition_matches(&cond(Field::Subject, Op::NotContains, "boleto, fatura"), &f, ME));
        assert!(condition_matches(&cond(Field::Subject, Op::StartsWith, "sua"), &f, ME));
        assert!(!condition_matches(&cond(Field::Subject, Op::Is, "fatura"), &f, ME));
    }

    #[test]
    fn domain_is_includes_subdomains_but_not_lookalikes() {
        let f = facts();
        assert!(condition_matches(&cond(Field::FromDomain, Op::Is, "loja.com.br"), &f, ME));
        assert!(condition_matches(&cond(Field::FromDomain, Op::Is, "news.loja.com.br"), &f, ME));
        let mut fake = facts();
        fake.from_address = "x@falsaloja.com.br".into();
        assert!(!condition_matches(&cond(Field::FromDomain, Op::Is, "loja.com.br"), &fake, ME));
    }

    #[test]
    fn yes_no_and_number_fields() {
        let f = facts();
        assert!(condition_matches(&cond(Field::List, Op::Yes, ""), &f, ME));
        assert!(condition_matches(&cond(Field::HasFiles, Op::Yes, ""), &f, ME));
        assert!(condition_matches(&cond(Field::Seen, Op::No, ""), &f, ME));
        assert!(condition_matches(&cond(Field::OnlyToMe, Op::Yes, ""), &f, ME));
        assert!(condition_matches(&cond(Field::SizeOverKb, Op::Over, "200"), &f, ME));
        assert!(!condition_matches(&cond(Field::SizeOverKb, Op::Over, "500"), &f, ME));
        assert!(condition_matches(&cond(Field::OlderThanDays, Op::Over, "14"), &f, ME));
        assert!(condition_matches(&cond(Field::NotNewestFromSender, Op::Yes, ""), &f, ME));
    }

    #[test]
    fn all_any_and_exceptions() {
        let f = facts();
        let mut s = spec(vec![cond(Field::Subject, Op::Contains, "fatura"), cond(Field::From, Op::Contains, "banco")], vec![Action::Star]);
        assert!(!matches(&s, &f, ME), "todas");
        s.match_all = false;
        assert!(matches(&s, &f, ME), "qualquer uma");
        s.exceptions = vec![cond(Field::HasFiles, Op::Yes, "")];
        assert!(!matches(&s, &f, ME), "exceto se");
        assert!(matches(&spec(vec![], vec![Action::MarkRead]), &f, ME), "sem condição vale para todos");
    }

    #[test]
    fn first_folder_action_wins_and_stop_ends() {
        let f = facts();
        let folder = |n: &str| Action::Move { folder: n.into() };
        let rules = vec![
            rule(1, spec(vec![cond(Field::List, Op::Yes, "")], vec![folder("leitura"), Action::MarkRead])),
            rule(2, spec(vec![], vec![Action::Archive, Action::Star])),
        ];
        let p = plan(&rules, &f, ME);
        assert_eq!(p.steps, vec![(1, vec![folder("leitura"), Action::MarkRead]), (2, vec![Action::Star])], "o arquivar da segunda é ignorado");
        assert_eq!(p.notify, Notify::Silent, "saiu da pasta: sem notificação de e-mail novo");

        let mut stopping = rules.clone();
        stopping[0].spec.stop = true;
        assert_eq!(plan(&stopping, &f, ME).steps.len(), 1);
    }

    #[test]
    fn notify_always_wins_over_silence() {
        let f = facts();
        let rules = vec![rule(1, spec(vec![], vec![Action::Silence])), rule(2, spec(vec![], vec![Action::NotifyAlways]))];
        assert_eq!(plan(&rules, &f, ME).notify, Notify::Always);
        let moved = vec![rule(1, spec(vec![], vec![Action::NotifyAlways, Action::Delete]))];
        assert_eq!(plan(&moved, &f, ME).notify, Notify::Always);
    }

    #[test]
    fn validation_refuses_broken_rules() {
        assert!(spec(vec![cond(Field::Subject, Op::Contains, "x")], vec![Action::Star]).validate().is_ok());
        assert!(spec(vec![], vec![]).validate().is_err(), "sem ação");
        assert!(spec(vec![cond(Field::Subject, Op::Yes, "")], vec![Action::Star]).validate().is_err(), "operador errado");
        assert!(spec(vec![cond(Field::Subject, Op::Contains, " , ")], vec![Action::Star]).validate().is_err(), "valor vazio");
        assert!(spec(vec![cond(Field::SizeOverKb, Op::Over, "abc")], vec![Action::Star]).validate().is_err());
        assert!(spec(vec![], vec![Action::Archive, Action::Delete]).validate().is_err(), "duas ações de pasta");
        let mut old = spec(vec![], vec![Action::Star]);
        old.v = 9;
        assert!(old.validate().is_err(), "versão desconhecida");
        assert!(spec(vec![cond(Field::OlderThanDays, Op::Over, "14")], vec![Action::Delete]).is_cleanup());
    }

    #[test]
    fn spec_json_roundtrips_and_reads_defaults() {
        let s = spec(vec![cond(Field::FromDomain, Op::Is, "loja.com")], vec![Action::Move { folder: "f".into() }, Action::Followup { days: 7 }]);
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<RuleSpec>(&json).unwrap(), s);
        let minimal: RuleSpec = serde_json::from_str(r#"{"matchAll":true,"conditions":[],"actions":[{"kind":"star"}]}"#).unwrap();
        assert_eq!((minimal.v, minimal.stop, minimal.folders.len()), (SPEC_VERSION, false, 0));
    }

    #[test]
    fn shadowing_rule_is_reported() {
        let f = facts();
        let mut top = rule(1, spec(vec![cond(Field::List, Op::Yes, "")], vec![Action::Star]));
        top.spec.stop = true;
        let new = spec(vec![cond(Field::Subject, Op::Contains, "fatura")], vec![Action::Archive]);
        assert_eq!(shadowed_by(std::slice::from_ref(&top), &new, std::slice::from_ref(&f), ME).map(|(r, n)| (r.id, n)), Some((1, 1)));
        top.spec.stop = false;
        assert!(shadowed_by(&[top], &new, &[f], ME).is_none());
    }
}
