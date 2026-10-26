//! Envio por SMTP com XOAUTH2 (US7, CA1–CA3). A mensagem é montada pela `lettre` (MIME,
//! cabeçalhos codificados, sem injeção de cabeçalho) e sai depois do prazo do "Desfazer envio"
//! (D3). O servidor grava a cópia em Enviados; aqui só sincronizamos a pasta depois.
//! Nunca vão para log: destinatários, assunto, corpo nem nomes de anexos. Só contagens e resultado.

use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use futures::FutureExt;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use mail_parser::{HeaderValue, MessageParser, MimeHeaders};
use rand::RngCore;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppError;
use crate::imap_client;
use crate::mailbox::Role;
use crate::session::{AccountHandle, AppState, Lane};
use crate::store::{self, DraftKind};
use crate::sync;

/// Prazo do "Desfazer envio" (o mesmo do toast).
pub const UNDO_SECS: u64 = 6;
/// Limite de tamanho do Gmail para a mensagem com anexos.
pub const MAX_SEND_BYTES: usize = 25 * 1024 * 1024;
const MAX_RECIPIENTS: usize = 100;
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const SMTP_TIMEOUT: Duration = Duration::from_secs(30);

/// Envios no prazo do "Desfazer". Cancelar tira o id daqui; quem sai do prazo ainda presente envia.
static WAITING: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Mutex::default);

/// Pedido de envio já validado (destinatários, assunto e tamanho do texto).
#[derive(Debug, Clone)]
pub struct Outgoing {
    pub account_id: String,
    pub kind: DraftKind,
    /// Mensagem respondida ou encaminhada (linha do banco) e a pasta de onde o frontend a abriu.
    pub original: Option<(i64, i64)>,
    pub to: Vec<Mailbox>,
    pub cc: Vec<Mailbox>,
    pub bcc: Vec<Mailbox>,
    pub subject: String,
    pub body: String,
    /// Posições (em `attachments()` da original) dos anexos que o encaminhamento leva.
    pub keep_attachments: Vec<usize>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SendFinished {
    send_id: String,
    account_id: String,
    ok: bool,
    error: Option<AppError>,
}

fn mailboxes(list: &[String]) -> Result<Vec<Mailbox>, AppError> {
    list.iter().map(|r| Mailbox::from_str(r.trim()).map_err(|_| AppError::InvalidRecipient)).collect()
}

/// Valida o pedido vindo do frontend. Nada é enviado aqui.
#[allow(clippy::too_many_arguments)]
pub fn validate(
    account_id: String,
    kind: DraftKind,
    original: Option<(i64, i64)>,
    to: &[String],
    cc: &[String],
    bcc: &[String],
    subject: &str,
    body: String,
    keep_attachments: Vec<usize>,
) -> Result<Outgoing, AppError> {
    let (to, cc, bcc) = (mailboxes(to)?, mailboxes(cc)?, mailboxes(bcc)?);
    let count = to.len() + cc.len() + bcc.len();
    if count == 0 {
        return Err(AppError::NoRecipients);
    }
    if count > MAX_RECIPIENTS || body.len() > MAX_BODY_BYTES || subject.chars().count() > 1000 {
        return Err(AppError::InvalidInput);
    }
    if (kind == DraftKind::New) != original.is_none() || (!keep_attachments.is_empty() && kind != DraftKind::Forward) {
        return Err(AppError::InvalidInput);
    }
    // Quebras de linha no assunto viram espaço (a lettre codifica o resto).
    let subject = subject.chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>().trim().to_owned();
    Ok(Outgoing { account_id, kind, original, to, cc, bcc, subject, body, keep_attachments })
}

/// Agenda o envio para depois do prazo do "Desfazer" e devolve o id para cancelar.
pub fn schedule(app: &AppHandle, outgoing: Outgoing) -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let send_id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    if let Ok(mut w) = WAITING.lock() {
        w.insert(send_id.clone());
    }
    let (app, id) = (app.clone(), send_id.clone());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(UNDO_SECS)).await;
        if !WAITING.lock().map(|mut w| w.remove(&id)).unwrap_or(false) {
            tracing::info!("envio desfeito");
            return;
        }
        let state = app.state::<AppState>();
        let account_id = outgoing.account_id.clone();
        let result = deliver(&state, &outgoing).await;
        let ok = result.is_ok();
        match &result {
            Ok(()) => tracing::info!(recipients = outgoing.to.len() + outgoing.cc.len() + outgoing.bcc.len(), "mensagem enviada"),
            Err(e) => tracing::warn!(kind = e.kind(), "mensagem não enviada"),
        }
        if ok {
            after_sent(&app, &state, &outgoing).await;
        } else if let Err(e) = &result {
            crate::notify::failure(&app, &account_id, "A mensagem não foi enviada".into(), e.to_string(), "O texto ficou no rascunho.".into());
        }
        let _ = app.emit("send-finished", SendFinished { send_id: id, account_id, ok, error: result.err() });
    });
    send_id
}

/// Desfaz um envio ainda no prazo. `false`: já saiu (ou não existe).
pub fn cancel(send_id: &str) -> bool {
    WAITING.lock().map(|mut w| w.remove(send_id)).unwrap_or(false)
}

/// Rascunho apagado e Enviados sincronizada.
async fn after_sent(app: &AppHandle, state: &AppState, outgoing: &Outgoing) {
    let (account_id, kind, original) = (outgoing.account_id.clone(), outgoing.kind, outgoing.original.map(|(m, _)| m));
    let sent = state
        .db
        .call(move |c| {
            store::delete_draft(c, &account_id, kind, original)?;
            store::folder_by_role(c, &account_id, Role::Sent)
        })
        .await;
    if let Ok(Some(folder)) = sent {
        sync::spawn_folder_sync(app, outgoing.account_id.clone(), folder.id);
    }
}

/// Mensagem original completa: do banco ou, sem corpo salvo, do servidor (pasta onde ela está agora).
async fn original_raw(state: &AppState, handle: &AccountHandle, message_id: i64, folder_hint: i64) -> Result<Vec<u8>, AppError> {
    let (cached, location) = state.db.call(move |c| Ok((store::load_body(c, message_id)?, store::location_of(c, message_id, folder_hint)?))).await?;
    if let Some(raw) = cached {
        return Ok(raw);
    }
    let (folder_id, uid) = location.ok_or(AppError::MessageMoving)?;
    let folder = state.db.call(move |c| store::get_folder(c, folder_id)).await?.ok_or(AppError::MessageNotFound)?;
    let uidvalidity = folder.uidvalidity.ok_or(AppError::MessageNotFound)?;
    let mailbox = folder.imap_name.clone();
    let raw = imap_client::with_session(state, handle, Lane::Interactive, |s| {
        let mailbox = mailbox.clone();
        async move {
            imap_client::select_checked(s, &mailbox, uidvalidity).await?;
            imap_client::fetch_raw(s, uid).await
        }
        .boxed()
    })
    .await?;
    Ok(raw.raw)
}

fn bracketed(id: &str) -> String {
    format!("<{}>", id.trim().trim_start_matches('<').trim_end_matches('>'))
}

/// `In-Reply-To` e `References` da resposta, a partir da original.
pub fn thread_headers(raw: &[u8]) -> Option<(String, String)> {
    let message = MessageParser::default().parse_headers(raw)?;
    let id = message.message_id().filter(|i| !i.trim().is_empty())?;
    let mut refs: Vec<String> = match message.references() {
        HeaderValue::Text(t) => vec![bracketed(t)],
        HeaderValue::TextList(list) => list.iter().map(|t| bracketed(t)).collect(),
        _ => Vec::new(),
    };
    refs.push(bracketed(id));
    Some((bracketed(id), refs.join(" ")))
}

/// Id novo com o domínio de quem envia (sem o nome do computador).
fn new_message_id(from: &Mailbox) -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("<{hex}@{}>", from.email.domain())
}

/// Monta a mensagem. `original` é a mensagem respondida ou encaminhada, quando há.
pub fn build(from: Mailbox, outgoing: &Outgoing, original: Option<&[u8]>) -> Result<Message, AppError> {
    let mut builder = Message::builder().message_id(Some(new_message_id(&from))).from(from).subject(outgoing.subject.clone()).date_now();
    for m in &outgoing.to {
        builder = builder.to(m.clone());
    }
    for m in &outgoing.cc {
        builder = builder.cc(m.clone());
    }
    for m in &outgoing.bcc {
        builder = builder.bcc(m.clone());
    }
    if matches!(outgoing.kind, DraftKind::Reply | DraftKind::ReplyAll) {
        if let Some((in_reply_to, references)) = original.and_then(thread_headers) {
            builder = builder.in_reply_to(in_reply_to).references(references);
        }
    }
    let text = SinglePart::plain(outgoing.body.clone());
    let mut total = outgoing.body.len();
    let mut parts = Vec::new();
    if !outgoing.keep_attachments.is_empty() {
        let parsed = original.and_then(|raw| MessageParser::default().parse(raw)).ok_or(AppError::MessageNotFound)?;
        for (_, part) in parsed.attachments().enumerate().filter(|(i, _)| outgoing.keep_attachments.contains(i)) {
            let bytes = part.contents().to_vec();
            total += bytes.len();
            if total > MAX_SEND_BYTES {
                return Err(AppError::AttachmentsTooLarge);
            }
            let mime = part
                .content_type()
                .map(|ct| match ct.subtype() {
                    Some(sub) => format!("{}/{}", ct.ctype(), sub),
                    None => ct.ctype().to_owned(),
                })
                .and_then(|m| ContentType::parse(&m).ok())
                .unwrap_or_else(|| ContentType::parse("application/octet-stream").expect("tipo fixo"));
            let name = part.attachment_name().unwrap_or("anexo").chars().filter(|c| !c.is_control()).collect::<String>();
            parts.push(Attachment::new(name).body(bytes, mime));
        }
    }
    let result = if parts.is_empty() {
        builder.singlepart(text)
    } else {
        let mut mixed = MultiPart::mixed().singlepart(text);
        for p in parts {
            mixed = mixed.singlepart(p);
        }
        builder.multipart(mixed)
    };
    result.map_err(|e| {
        tracing::warn!(error = %e, "mensagem não pôde ser montada");
        AppError::InvalidInput
    })
}

async fn deliver(state: &AppState, outgoing: &Outgoing) -> Result<(), AppError> {
    let id = outgoing.account_id.clone();
    let (account, sender, (host, port, security)) = state
        .db
        .call(move |c| Ok((store::require_account(c, &id)?, store::sender_name(c, &id)?, store::smtp_server(c, &id)?)))
        .await?;
    let handle = state.handle(&account.id).await?;
    let original = match outgoing.original {
        Some((message_id, folder_hint)) => Some(original_raw(state, &handle, message_id, folder_hint).await?),
        None => None,
    };
    let address = account.email.parse().map_err(|_| AppError::InvalidRecipient)?;
    let from = Mailbox::new(sender.filter(|n| !n.trim().is_empty()), address);
    let message = build(from, outgoing, original.as_deref())?;

    let mut force_refresh = false;
    for attempt in 0..2 {
        let token = state.access(&handle, force_refresh).await?;
        let builder = if security == "starttls" { AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host) } else { AsyncSmtpTransport::<Tokio1Executor>::relay(&host) };
        let transport = builder
            .map_err(|e| {
                tracing::warn!(error = %e, "servidor de envio inválido");
                AppError::SmtpConnection
            })?
            .port(port)
            // A lettre guarda a própria cópia do token; a nossa é zerada no fim da volta.
            .credentials(Credentials::new(account.email.clone(), token.to_string()))
            .authentication(vec![Mechanism::Xoauth2])
            .timeout(Some(SMTP_TIMEOUT))
            .build();
        match transport.send(message.clone()).await {
            Ok(_) => return Ok(()),
            Err(e) if attempt == 0 && e.status().is_some_and(|c| c.to_string() == "535") => {
                tracing::info!("servidor de envio recusou o token; renovando");
                force_refresh = true;
            }
            Err(e) if e.is_permanent() => {
                tracing::warn!(status = ?e.status().map(|c| c.to_string()), "servidor de envio recusou a mensagem");
                return Err(AppError::SendRejected);
            }
            Err(e) => {
                tracing::warn!(status = ?e.status().map(|c| c.to_string()), timeout = e.is_timeout(), "falha de conexão com o servidor de envio");
                return Err(AppError::SmtpConnection);
            }
        }
    }
    Err(AppError::SendRejected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outgoing(kind: DraftKind) -> Outgoing {
        validate(
            "a".into(),
            kind,
            (kind != DraftKind::New).then_some((1, 1)),
            &["Ana Ribeiro <ana@lumen.com.br>".into()],
            &["equipe@lumen.com.br".into()],
            &["oculto@lumen.com.br".into()],
            "Re: Contrato\r\nBcc: invasor@x.com",
            "Oi, Ana.\nSegue.".into(),
            Vec::new(),
        )
        .unwrap()
    }

    fn from() -> Mailbox {
        Mailbox::new(Some("João Silva".into()), "joao@gmail.com".parse().unwrap())
    }

    #[test]
    fn validation_requires_valid_recipients() {
        let v = |to: &[String]| validate("a".into(), DraftKind::New, None, to, &[], &[], "s", "b".into(), Vec::new());
        assert_eq!(v(&[]).unwrap_err(), AppError::NoRecipients);
        assert_eq!(v(&["ana@".into()]).unwrap_err(), AppError::InvalidRecipient);
        assert!(v(&["Ana <ana@x.com>".into(), "b@x.com".into()]).is_ok());
        assert_eq!(validate("a".into(), DraftKind::Reply, None, &["a@x.com".into()], &[], &[], "s", "b".into(), Vec::new()).unwrap_err(), AppError::InvalidInput);
    }

    #[test]
    fn message_has_no_header_injection_and_hides_bcc() {
        let original = b"Message-ID: <abc@lumen.com.br>\r\nReferences: <r1@x>\r\nSubject: Contrato\r\n\r\nx";
        let m = build(from(), &outgoing(DraftKind::Reply), Some(original)).unwrap();
        let text = String::from_utf8(m.formatted()).unwrap();
        assert!(text.contains("Subject: Re: Contrato  Bcc: invasor@x.com"), "{text}");
        assert!(!text.contains("\r\nBcc:"), "Cco não vai nos cabeçalhos");
        assert!(text.contains("In-Reply-To: <abc@lumen.com.br>"));
        assert!(text.contains("References: <r1@x> <abc@lumen.com.br>"));
        assert!(text.contains("@gmail.com>"), "Message-ID com o domínio de quem envia");
        let envelope = m.envelope();
        assert_eq!(envelope.to().len(), 3, "Cco continua no envelope");
    }

    #[test]
    fn forward_keeps_only_chosen_attachments() {
        let raw = b"From: a@x.com\r\nSubject: s\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=b\r\n\r\n\
--b\r\nContent-Type: text/plain\r\n\r\nOi\r\n\
--b\r\nContent-Type: application/pdf; name=um.pdf\r\nContent-Disposition: attachment; filename=um.pdf\r\n\r\nPDF1\r\n\
--b\r\nContent-Type: text/csv; name=dois.csv\r\nContent-Disposition: attachment; filename=dois.csv\r\n\r\nCSV2\r\n\
--b--\r\n";
        let mut o = outgoing(DraftKind::Forward);
        o.keep_attachments = vec![1];
        let text = String::from_utf8(build(from(), &o, Some(raw)).unwrap().formatted()).unwrap();
        assert!(text.contains("dois.csv") && !text.contains("um.pdf"), "{text}");
    }

    #[test]
    fn thread_headers_without_message_id() {
        assert_eq!(thread_headers(b"Subject: s\r\n\r\nx"), None);
        assert_eq!(thread_headers(b"Message-ID: <a@b>\r\n\r\nx"), Some(("<a@b>".into(), "<a@b>".into())));
    }
}
