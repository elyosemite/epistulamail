//! Modelos de resposta e encaminhamento (US4, CA8): destinatários, assunto e a mensagem original,
//! montados a partir da mensagem salva. Nada daqui vai para log.

use mail_parser::{Address, MessageParser};
use serde::Serialize;

use crate::store::DraftKind;

const REPLY_PREFIXES: &[&str] = &["re:", "res:", "ref:", "aw:", "sv:"];
const FORWARD_PREFIXES: &[&str] = &["fw:", "fwd:", "enc:", "tr:", "wg:"];

/// Tira um prefixo conhecido (sem diferenciar maiúsculas), uma vez.
fn strip_prefix<'a>(subject: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    let lower = subject.to_lowercase();
    prefixes
        .iter()
        .find(|p| lower.starts_with(*p))
        .map(|p| subject[p.len()..].trim_start())
}

/// "Re: <assunto>", sem repetir o prefixo quando o assunto já é uma resposta.
pub fn reply_subject(subject: &str) -> String {
    let subject = subject.trim();
    format!("Re: {}", strip_prefix(subject, REPLY_PREFIXES).unwrap_or(subject))
}

/// "Fw: <assunto>", sem repetir o prefixo quando o assunto já é um encaminhamento.
pub fn forward_subject(subject: &str) -> String {
    let subject = subject.trim();
    format!("Fw: {}", strip_prefix(subject, FORWARD_PREFIXES).unwrap_or(subject))
}

fn format_person(name: Option<&str>, address: &str) -> String {
    match name.map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => format!("{n} <{address}>"),
        None => address.to_owned(),
    }
}

fn addresses(list: Option<&Address<'_>>) -> Vec<(String, String)> {
    list.map(|a| {
        a.iter()
            .filter_map(|addr| {
                let address = addr.address()?.trim().to_owned();
                (!address.is_empty()).then(|| (address, format_person(addr.name(), addr.address().unwrap_or_default().trim())))
            })
            .collect()
    })
    .unwrap_or_default()
}

/// Remove a própria conta e repetidos (comparando o endereço sem diferenciar maiúsculas).
fn dedupe(list: Vec<(String, String)>, own: &str, seen: &mut Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for (address, display) in list {
        let key = address.to_lowercase();
        if key == own || seen.contains(&key) {
            continue;
        }
        seen.push(key);
        out.push(display);
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Quote {
    pub from: String,
    /// Data RFC 3339; o frontend formata.
    pub sent: String,
    pub to: String,
    pub cc: String,
    pub subject: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Template {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub quote: Option<Quote>,
}

/// Modelo de resposta, resposta a todos ou encaminhamento. `own_email` é o e-mail da conta, que
/// nunca entra como destinatário de "responder a todos".
pub fn template(raw: &[u8], kind: DraftKind, own_email: &str) -> Template {
    let own = own_email.trim().to_lowercase();
    let Some(message) = MessageParser::default().parse(raw) else {
        return Template { to: Vec::new(), cc: Vec::new(), subject: String::new(), quote: None };
    };
    let subject = message.subject().unwrap_or_default().trim().to_owned();
    let from = addresses(message.from());
    let reply_target = {
        let reply_to = addresses(message.reply_to());
        if reply_to.is_empty() { from.clone() } else { reply_to }
    };
    match kind {
        DraftKind::Reply => {
            let mut seen = Vec::new();
            let mut to = dedupe(reply_target.clone(), "", &mut seen);
            // Respondendo à própria mensagem (em Enviados), responde para quem a recebeu.
            if reply_target.iter().all(|(a, _)| a.to_lowercase() == own) {
                to = dedupe(addresses(message.to()), &own, &mut Vec::new());
            }
            Template { to, cc: Vec::new(), subject: reply_subject(&subject), quote: None }
        }
        DraftKind::ReplyAll => {
            let mut seen = Vec::new();
            let mut to = dedupe(reply_target, &own, &mut seen);
            to.extend(dedupe(addresses(message.to()), &own, &mut seen));
            let cc = dedupe(addresses(message.cc()), &own, &mut seen);
            Template { to, cc, subject: reply_subject(&subject), quote: None }
        }
        DraftKind::Forward | DraftKind::New => {
            let join = |l: Vec<(String, String)>| l.into_iter().map(|(_, d)| d).collect::<Vec<_>>().join("; ");
            let quote = (kind == DraftKind::Forward).then(|| Quote {
                from: join(from),
                sent: message.date().map(|d| d.to_rfc3339()).unwrap_or_default(),
                to: join(addresses(message.to())),
                cc: join(addresses(message.cc())),
                subject: subject.clone(),
                text: message.body_text(0).map(|t| t.into_owned()).unwrap_or_default(),
            });
            let subject = if kind == DraftKind::Forward { forward_subject(&subject) } else { String::new() };
            Template { to: Vec::new(), cc: Vec::new(), subject, quote }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MESSAGE: &[u8] = b"From: Marina Couto <marina@lumen.com.br>\r\n\
To: Joao <JOAO@gmail.com>, Rafael Dias <rafael@empresa.com>, marina@lumen.com.br\r\n\
Cc: equipe@lumen.com.br, rafael@empresa.com\r\n\
Subject: RE: Revisao do contrato\r\n\
Date: Thu, 23 Oct 2026 10:42:00 -0300\r\n\
Content-Type: text/plain; charset=utf-8\r\n\r\n\
Segue a versao 3.\r\n";

    #[test]
    fn subjects_get_a_single_prefix() {
        assert_eq!(reply_subject("Revisão"), "Re: Revisão");
        assert_eq!(reply_subject("Re: Revisão"), "Re: Revisão");
        assert_eq!(reply_subject("RE:Revisão"), "Re: Revisão");
        assert_eq!(reply_subject("Res: Revisão"), "Re: Revisão");
        assert_eq!(reply_subject(""), "Re: ");
        assert_eq!(forward_subject("Revisão"), "Fw: Revisão");
        assert_eq!(forward_subject("Fwd: Revisão"), "Fw: Revisão");
        assert_eq!(forward_subject("FW: Revisão"), "Fw: Revisão");
        assert_eq!(forward_subject("Re: Revisão"), "Fw: Re: Revisão");
    }

    #[test]
    fn reply_goes_to_the_sender_or_reply_to() {
        let t = template(MESSAGE, DraftKind::Reply, "joao@gmail.com");
        assert_eq!(t.to, ["Marina Couto <marina@lumen.com.br>"]);
        assert!(t.cc.is_empty());
        assert_eq!(t.subject, "Re: Revisao do contrato");

        let with_reply_to = b"From: a@x.com\r\nReply-To: Lista <lista@x.com>\r\nSubject: s\r\n\r\nx";
        assert_eq!(template(with_reply_to, DraftKind::Reply, "eu@x.com").to, ["Lista <lista@x.com>"]);
    }

    #[test]
    fn reply_all_skips_the_own_account_and_duplicates() {
        let t = template(MESSAGE, DraftKind::ReplyAll, "Joao@Gmail.com");
        assert_eq!(t.to, ["Marina Couto <marina@lumen.com.br>", "Rafael Dias <rafael@empresa.com>"]);
        assert_eq!(t.cc, ["equipe@lumen.com.br"]);
    }

    #[test]
    fn replying_to_own_sent_message_targets_its_recipients() {
        let sent = b"From: Eu <joao@gmail.com>\r\nTo: Ana <ana@x.com>\r\nSubject: Oi\r\n\r\nx";
        assert_eq!(template(sent, DraftKind::Reply, "joao@gmail.com").to, ["Ana <ana@x.com>"]);
    }

    #[test]
    fn forward_quotes_the_original() {
        let t = template(MESSAGE, DraftKind::Forward, "joao@gmail.com");
        assert!(t.to.is_empty());
        assert_eq!(t.subject, "Fw: RE: Revisao do contrato");
        let q = t.quote.unwrap();
        assert_eq!(q.from, "Marina Couto <marina@lumen.com.br>");
        assert_eq!(q.to, "Joao <JOAO@gmail.com>; Rafael Dias <rafael@empresa.com>; marina@lumen.com.br");
        assert_eq!(q.subject, "RE: Revisao do contrato");
        assert!(q.sent.starts_with("2026-10-23T10:42:00"));
        assert!(q.text.contains("Segue a versao 3."));
    }

    #[test]
    fn garbage_never_panics() {
        for kind in [DraftKind::Reply, DraftKind::ReplyAll, DraftKind::Forward] {
            let _ = template(b"\xff\xfe nada", kind, "a@b.c");
        }
    }
}
