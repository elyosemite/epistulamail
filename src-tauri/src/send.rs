//! Envio por SMTP com XOAUTH2 (US7, CA1–CA3). A mensagem é montada pela `lettre` (MIME,
//! cabeçalhos codificados, sem injeção de cabeçalho) e sai depois do prazo do "Desfazer envio"
//! (D3). O servidor grava a cópia em Enviados; aqui só sincronizamos a pasta depois.
//! Nunca vão para log: destinatários, assunto, corpo nem nomes de anexos. Só contagens e resultado.
//! Com OpenPGP (US13), o corpo sai cifrado e assinado (PGP/MIME) ou só assinado, com o cabeçalho
//! Autocrypt quando a conta tem chave.

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
    /// Criptografar e assinar (US13, CA6), com OpenPGP ou, com `smime`, com S/MIME (US14).
    pub encrypt: bool,
    pub sign: bool,
    pub smime: bool,
}

/// O que a criptografia precisa para montar a mensagem (US13).
pub struct Crypto {
    pub encrypt: bool,
    pub sign: bool,
    /// Minha chave e as dos destinatários; só quando cifra ou assina com OpenPGP.
    pub keys: Option<crate::e2ee::SendKeys>,
    /// Meu certificado e os dos destinatários; só quando cifra ou assina com S/MIME.
    pub smime: Option<crate::e2ee::SmimeKeys>,
    /// Minha chave pública (binária), para o Autocrypt e o anexo.
    pub own_public: Option<Vec<u8>>,
    pub settings: crate::pgp_store::CryptoSettings,
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
    Ok(Outgoing { account_id, kind, original, to, cc, bcc, subject, body, keep_attachments, encrypt: false, sign: false, smime: false })
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
#[cfg(test)]
pub fn build(from: Mailbox, outgoing: &Outgoing, original: Option<&[u8]>) -> Result<Message, AppError> {
    build_with(from, outgoing, original, None)
}

/// Monta a mensagem, cifrada, assinada ou com Autocrypt quando `crypto` pede.
pub fn build_with(from: Mailbox, outgoing: &Outgoing, original: Option<&[u8]>, crypto: Option<&Crypto>) -> Result<Message, AppError> {
    let encrypting = crypto.is_some_and(|c| c.encrypt);
    // S/MIME mantém o assunto: Outlook e Apple Mail não leem o assunto protegido (D2 da US14).
    let hide_subject = encrypting && crypto.is_some_and(|c| c.settings.encrypt_subject && c.smime.is_none());
    let from_address = from.email.to_string();
    let outer_subject = if hide_subject { crate::pgp_mime::HIDDEN_SUBJECT.to_owned() } else { outgoing.subject.clone() };
    let mut builder = Message::builder().message_id(Some(new_message_id(&from))).from(from).subject(outer_subject).date_now();
    if let Some(c) = crypto {
        if let (true, Some(public)) = (c.settings.autocrypt, &c.own_public) {
            builder = builder.raw_header(crate::pgp_mime::autocrypt_header(&from_address, public));
        }
    }
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
    // Assinada com OpenPGP: a minha chave pública vai junto, para responderem cifrado (US13, CA1).
    if let Some(c) = crypto {
        if let (true, true, Some(keys)) = (c.encrypt || c.sign, c.settings.attach_key, &c.keys) {
            let public = crate::openpgp::armored_public(&keys.mine.to_public_key())?;
            let id = crate::openpgp::info(&keys.mine.to_public_key()).fingerprint;
            let name = format!("OpenPGP_0x{}.asc", &id[id.len().saturating_sub(16)..]);
            parts.push(Attachment::new(name).body(public, ContentType::parse("application/pgp-keys").expect("tipo fixo")));
        }
    }
    let inner = if parts.is_empty() {
        crate::pgp_mime::Inner::Single(text)
    } else {
        let mut mixed = MultiPart::mixed().singlepart(text);
        for p in parts {
            mixed = mixed.singlepart(p);
        }
        crate::pgp_mime::Inner::Multi(mixed)
    };
    if let Some((c, keys)) = crypto.and_then(|c| c.smime.as_ref().map(|k| (c, k))) {
        // Assinada sempre que cifrada; cifrar envolve a parte já assinada (RFC 8551, seção 3.7).
        let signed = crate::pgp_mime::canonical(&inner.formatted());
        let signature = crate::smime::sign_detached(&signed, &keys.cert, &keys.key, &keys.chain)?;
        let entity = crate::smime_mime::signed_entity(&signed, &signature);
        let result = if c.encrypt {
            let der = crate::smime::encrypt(&entity.bytes(), &keys.recipients)?;
            let (content_type, body, [(name, value)]) = crate::smime_mime::enveloped_entity(&der);
            builder.header(content_type).raw_header(lettre::message::header::HeaderValue::new(lettre::message::header::HeaderName::new_from_ascii_str(name), value.to_owned())).body(body)
        } else {
            let (content_type, body) = entity.into_lettre();
            builder.header(content_type).body(body)
        };
        return result.map_err(|e| {
            tracing::warn!(error = %e, "mensagem não pôde ser montada");
            AppError::InvalidInput
        });
    }
    let result = match crypto.and_then(|c| c.keys.as_ref().map(|k| (c, k))) {
        Some((c, keys)) if c.encrypt => {
            let mut body = inner.formatted();
            if hide_subject {
                body = crate::pgp_mime::protect_subject(&body, &outgoing.subject);
            }
            let cipher = crate::openpgp::encrypt(&body, &keys.recipients, Some(&keys.mine))?;
            builder.multipart(crate::pgp_mime::encrypted_body(cipher))
        }
        Some((c, keys)) if c.sign => {
            let signed = crate::pgp_mime::canonical(&inner.formatted());
            let signature = crate::openpgp::sign_detached(&signed, &keys.mine)?;
            let (content_type, body) = crate::pgp_mime::signed_body(&signed, &signature);
            builder.header(content_type).body(body)
        }
        _ => match inner {
            crate::pgp_mime::Inner::Single(p) => builder.singlepart(p),
            crate::pgp_mime::Inner::Multi(p) => builder.multipart(p),
        },
    };
    result.map_err(|e| {
        tracing::warn!(error = %e, "mensagem não pôde ser montada");
        AppError::InvalidInput
    })
}

/// Configurações e chaves da conta para o envio. Cifrar sem a chave de algum destinatário falha
/// antes de qualquer coisa sair (US13, regra 3).
async fn load_crypto(state: &AppState, outgoing: &Outgoing) -> Result<Crypto, AppError> {
    let (id, encrypt, sign, smime) = (outgoing.account_id.clone(), outgoing.encrypt, outgoing.sign, outgoing.smime);
    let roots = crate::smime::system_roots();
    let recipients: Vec<String> = outgoing.to.iter().chain(&outgoing.cc).chain(&outgoing.bcc).map(|m| m.email.to_string().to_lowercase()).collect();
    let loaded = state
        .db
        .call(move |c| {
            let settings = crate::pgp_store::crypto_settings(c, &id)?;
            let wants = encrypt || sign;
            let keys = if wants && !smime { Some(crate::e2ee::send_keys(c, &id, &recipients, encrypt)?) } else { None };
            let certs = if wants && smime { Some(crate::e2ee::smime_keys(c, &id, &recipients, encrypt, roots)?) } else { None };
            let own_public = match &settings.pgp_key {
                Some(fp) => crate::pgp_store::public_key(c, fp)?,
                None => None,
            };
            Ok((settings, keys, certs, own_public))
        })
        .await?;
    let (settings, keys, certs, own_public) = loaded;
    tracing::info!(encrypt, sign, smime, "envio com criptografia preparado");
    Ok(Crypto { encrypt, sign, keys, smime: certs, own_public, settings })
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
    let crypto = load_crypto(state, outgoing).await?;
    let message = build_with(from, outgoing, original.as_deref(), Some(&crypto))?;

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
    fn read_receipts_are_never_sent() {
        // US10, CA8: responder a quem pediu confirmação de leitura não confirma nada.
        let original = b"Message-ID: <abc@lumen.com.br>
Disposition-Notification-To: rastreio@lumen.com.br
Subject: Contrato

x";
        let m = build(from(), &outgoing(DraftKind::Reply), Some(original)).unwrap();
        let text = String::from_utf8_lossy(&m.formatted()).to_ascii_lowercase();
        assert!(!text.contains("disposition-notification"), "{text}");
        assert!(!text.contains("rastreio@lumen.com.br"));
        // Nenhum módulo do app monta uma confirmação de leitura (o relatório da RFC 8098).
        let report = ["multipart", "/report"].concat();
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let source = std::fs::read_to_string(&path).unwrap().to_ascii_lowercase();
            assert!(!source.contains(&report), "{} monta {report}", path.display());
        }
    }

    #[test]
    fn thread_headers_without_message_id() {
        assert_eq!(thread_headers(b"Subject: s\r\n\r\nx"), None);
        assert_eq!(thread_headers(b"Message-ID: <a@b>\r\n\r\nx"), Some(("<a@b>".into(), "<a@b>".into())));
    }
    #[test]
    fn smime_message_is_signed_inside_the_envelope_and_keeps_the_subject() {
        use crate::smime::tests::{person, test_ca};
        use crate::smime::{self, CertKind, Checked};
        use crate::smime_mime::{self, Found};
        let (ca, ca_key) = test_ca();
        let (me, my_key) = person("eu@x.com", CertKind::Rsa3072, &ca, &ca_key);
        let (ana, ana_key) = person("ana@lumen.com.br", CertKind::P256, &ca, &ca_key);
        let keys = crate::e2ee::SmimeKeys { cert: me.clone(), key: my_key, chain: vec![ca.clone()], recipients: vec![ana.clone(), me.clone()] };
        let crypto = Crypto { encrypt: true, sign: true, keys: None, smime: Some(keys), own_public: None, settings: crate::pgp_store::CryptoSettings::default() };
        let from = Mailbox::new(None, "eu@x.com".parse().unwrap());
        let raw = build_with(from, &outgoing(DraftKind::New), None, Some(&crypto)).unwrap().formatted();
        let text = String::from_utf8_lossy(&raw);
        assert!(text.contains("smime-type=enveloped-data") && !text.contains("Oi, Ana"));
        assert!(text.contains("Subject: Re: Contrato"), "o assunto fica no S/MIME");
        let Found::Enveloped(der) = smime_mime::find(&raw) else { panic!("esperava cifrada") };
        let inner = smime::decrypt(&der, &[(ana, ana_key)]).unwrap();
        let Found::Signed { content, signature, .. } = smime_mime::find(&inner) else { panic!("esperava assinada por dentro") };
        assert!(matches!(smime::verify(&signature, Some(&content), &[]), Checked::Good { .. }));
        assert!(String::from_utf8_lossy(&content).contains("Oi, Ana."));
    }

    fn pgp_crypto(encrypt: bool, me: &pgp::composed::SignedSecretKey, ana: &pgp::composed::SignedPublicKey) -> Crypto {
        let recipients = if encrypt { vec![ana.clone(), me.to_public_key()] } else { Vec::new() };
        let settings = crate::pgp_store::CryptoSettings { attach_key: true, autocrypt: true, encrypt_subject: true, ..Default::default() };
        let own_public = Some(crate::openpgp::public_bytes(&me.to_public_key()).unwrap());
        Crypto { encrypt, sign: true, keys: Some(crate::e2ee::SendKeys { mine: me.clone(), recipients }), smime: None, own_public, settings }
    }

    #[test]
    fn openpgp_message_hides_subject_and_reads_back_with_the_attached_key() {
        use crate::openpgp::{self, KeyKind, Verification};
        let me = openpgp::generate(KeyKind::Curve25519, "Eu", "eu@x.com").unwrap();
        let ana = openpgp::generate(KeyKind::Curve25519, "Ana", "ana@lumen.com.br").unwrap();
        let from = Mailbox::new(None, "eu@x.com".parse().unwrap());
        let mut o = outgoing(DraftKind::New);
        o.body = "Olá, Ana. Revisão até sexta.".into();
        let raw = build_with(from, &o, None, Some(&pgp_crypto(true, &me, &ana.to_public_key()))).unwrap().formatted();
        let text = String::from_utf8_lossy(&raw);
        assert!(text.contains("multipart/encrypted") && text.contains("Subject: ..."), "{text}");
        assert!(!text.contains("Contrato") && !text.contains("Revis") && !text.contains("OpenPGP_0x"), "assunto, corpo e anexo vão cifrados");
        // De fora, só a chave do Autocrypt; o anexo com a chave está dentro da parte cifrada.
        assert_eq!(crate::e2ee::incoming_keys(&raw, "eu@x.com").iter().map(|(_, o)| *o).collect::<Vec<_>>(), ["autocrypt"]);

        // Ana lê sem conhecer a minha chave: a assinatura confere com a chave anexada, que ela guarda.
        let found = crate::pgp_mime::find(&raw);
        let (opened, inside) = crate::e2ee::read_pgp(raw.clone(), "eu@x.com", found.clone(), &[], std::slice::from_ref(&ana));
        let sec = opened.security.unwrap();
        assert!(sec.encrypted && sec.decrypted);
        let fp = openpgp::info(&me.to_public_key()).fingerprint;
        assert_eq!(sec.signature, Verification::Good { fingerprint: fp.clone() });
        assert_eq!(opened.subject.as_deref(), Some("Re: Contrato  Bcc: invasor@x.com"));
        assert_eq!(inside.len(), 1);
        assert_eq!(openpgp::info(&inside[0].0).fingerprint, fp);
        let parsed = MessageParser::default().parse(&opened.raw).unwrap();
        assert!(parsed.body_text(0).is_some_and(|t| t.contains("Olá, Ana. Revisão até sexta.")));
        assert_eq!(parsed.from().and_then(|f| f.first()).and_then(|a| a.address()), Some("eu@x.com"));

        // A minha cópia em Enviados abre com a minha chave; quem não está entre os destinatários não abre.
        assert!(crate::e2ee::read_pgp(raw.clone(), "eu@x.com", found.clone(), &[], std::slice::from_ref(&me)).0.security.unwrap().decrypted);
        let other = openpgp::generate(KeyKind::Curve25519, "Outro", "outro@x.com").unwrap();
        let (closed, inside) = crate::e2ee::read_pgp(raw.clone(), "eu@x.com", found.clone(), &[], &[other]);
        assert!(!closed.security.unwrap().decrypted && inside.is_empty());
        // A chave anexada de outra pessoa não vale para quem diz ser o remetente.
        let (_, inside) = crate::e2ee::read_pgp(raw, "golpe@x.com", found, &[], std::slice::from_ref(&ana));
        assert!(inside.is_empty());
    }

    #[test]
    fn openpgp_signed_message_keeps_accents_and_verifies_after_sending() {
        use crate::openpgp::{self, KeyKind, Verification};
        let me = openpgp::generate(KeyKind::Curve25519, "Eu", "eu@x.com").unwrap();
        let ana = openpgp::generate(KeyKind::Curve25519, "Ana", "ana@lumen.com.br").unwrap().to_public_key();
        let from = Mailbox::new(None, "eu@x.com".parse().unwrap());
        let mut o = outgoing(DraftKind::New);
        o.body = "Olá, Ana.\nAção: revisão até sexta — às 10h.\n".into();
        let raw = build_with(from, &o, None, Some(&pgp_crypto(false, &me, &ana))).unwrap().formatted();
        assert!(raw.is_ascii(), "a mensagem assinada vai em 7 bits");
        let text = String::from_utf8_lossy(&raw);
        assert!(text.contains("multipart/signed") && text.contains("Subject: Re: Contrato"), "só assinada, o assunto fica");
        assert!(text.contains("OpenPGP_0x"), "a minha chave vai anexada");
        let found = crate::pgp_mime::find(&raw);
        let (opened, _) = crate::e2ee::read_pgp(raw.clone(), "eu@x.com", found, &[me.to_public_key()], &[]);
        let sec = opened.security.unwrap();
        assert!(!sec.encrypted);
        assert!(matches!(sec.signature, Verification::Good { .. }), "{:?}", sec.signature);
        let parsed = MessageParser::default().parse(&opened.raw).unwrap();
        assert!(parsed.body_text(0).is_some_and(|t| t.contains("Ação: revisão até sexta — às 10h.")));
        // Sem a chave, assinatura de chave desconhecida; e a anexada chega de fora.
        let (unknown, _) = crate::e2ee::read_pgp(raw.clone(), "eu@x.com", crate::pgp_mime::find(&raw), &[], &[]);
        assert!(matches!(unknown.security.unwrap().signature, Verification::UnknownKey { .. }));
        assert!(crate::e2ee::incoming_keys(&raw, "eu@x.com").iter().any(|(_, o)| *o == "attachment"));
    }
}
