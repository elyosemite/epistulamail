//! Arquivos do Outlook (US15): `.pst` (MS-PST, pelo leitor da Microsoft, só leitura) e `.msg`
//! (MS-OXMSG sobre Compound File). O Outlook guarda propriedades MAPI, não MIME: cada mensagem é
//! remontada em MIME, com os cabeçalhos originais da internet quando o Outlook os guardou. Nada
//! do conteúdo, nomes de pastas ou caminho vai para o log.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use outlook_pst::ltp::prop_context::PropertyValue;
use outlook_pst::ltp::table_context::TableContext;
use outlook_pst::messaging::attachment::{AnsiAttachment, Attachment, AttachmentData, UnicodeAttachment};
use outlook_pst::messaging::folder::Folder;
use outlook_pst::messaging::message::{AnsiMessage, Message, UnicodeMessage};
use outlook_pst::messaging::store::{AnsiStore, EntryId, Store, UnicodeStore};
use outlook_pst::ndb::node_id::NodeId;
use outlook_pst::{AnsiPstFile, UnicodePstFile};

use crate::error::AppError;
use crate::mail_import::{role_for, Failure, MAX_MESSAGE_BYTES};
use crate::mailbox::Role;
use crate::message::MessageFlags;

// Propriedades MAPI (MS-OXPROPS).
const SUBJECT: u16 = 0x0037;
const MESSAGE_CLASS: u16 = 0x001A;
const CLIENT_SUBMIT_TIME: u16 = 0x0039;
const SENT_REPRESENTING_NAME: u16 = 0x0042;
const SENT_REPRESENTING_EMAIL: u16 = 0x0065;
const TRANSPORT_HEADERS: u16 = 0x007D;
const SENDER_NAME: u16 = 0x0C1A;
const SENDER_EMAIL: u16 = 0x0C1F;
const DELIVERY_TIME: u16 = 0x0E06;
const MESSAGE_FLAGS: u16 = 0x0E07;
const BODY: u16 = 0x1000;
const HTML: u16 = 0x1013;
const INTERNET_MESSAGE_ID: u16 = 0x1035;
const INTERNET_REFERENCES: u16 = 0x1039;
const IN_REPLY_TO: u16 = 0x1042;
const FLAG_STATUS: u16 = 0x1090;
const RECIPIENT_TYPE: u16 = 0x0C15;
const DISPLAY_NAME: u16 = 0x3001;
const EMAIL_ADDRESS: u16 = 0x3003;
const CONTAINER_CLASS: u16 = 0x3613;
const ATTACH_FILENAME: u16 = 0x3704;
const ATTACH_LONG_FILENAME: u16 = 0x3707;
const ATTACH_MIME_TAG: u16 = 0x370E;
const ATTACH_CONTENT_ID: u16 = 0x3712;
const SMTP_ADDRESS: u16 = 0x39FE;
const INTERNET_CPID: u16 = 0x3FDE;
const SENDER_SMTP: u16 = 0x5D01;
const SENT_REPRESENTING_SMTP: u16 = 0x5D02;

const MSGFLAG_READ: i64 = 0x1;
const FLAG_STATUS_FLAGGED: i64 = 2;
const MAX_EMBEDDED_DEPTH: usize = 3;

/// Uma mensagem como o Outlook a guarda, já com os valores que importam.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MapiMessage {
    pub subject: String,
    pub from_name: String,
    pub from_address: String,
    pub to: Vec<(String, String)>,
    pub cc: Vec<(String, String)>,
    /// Segundos desde 1970.
    pub date: Option<i64>,
    /// Cabeçalhos da internet guardados pelo Outlook (mensagens recebidas).
    pub transport_headers: Option<String>,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
    pub text: Option<String>,
    /// HTML como o Outlook guardou (bytes) e o charset deles.
    pub html: Option<(Vec<u8>, String)>,
    pub attachments: Vec<MapiAttachment>,
    pub read: bool,
    pub flagged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapiAttachment {
    pub name: String,
    pub mime: String,
    pub content_id: Option<String>,
    pub data: AttachmentBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentBody {
    Bytes(Vec<u8>),
    /// Mensagem anexada (encaminhada como anexo), já em MIME.
    Message(Vec<u8>),
}

// ---------------------------------------------------------------- valores

/// Os valores de uma mensagem, uma pessoa ou um anexo, pelo id da propriedade.
pub trait Props {
    fn text(&self, id: u16) -> Option<String>;
    fn bytes(&self, id: u16) -> Option<Vec<u8>>;
    fn int(&self, id: u16) -> Option<i64>;
}

fn value_text(v: &PropertyValue) -> Option<String> {
    match v {
        PropertyValue::String8(s) => Some(s.to_string()),
        PropertyValue::Unicode(s) => Some(s.to_string()),
        _ => None,
    }
    .map(|s| s.trim_end_matches('\0').to_owned())
    .filter(|s| !s.is_empty())
}

fn value_bytes(v: &PropertyValue) -> Option<Vec<u8>> {
    match v {
        PropertyValue::Binary(b) => Some(b.buffer().to_vec()),
        PropertyValue::String8(s) => Some(s.buffer().to_vec()),
        PropertyValue::Unicode(s) => Some(s.to_string().into_bytes()),
        _ => None,
    }
}

fn value_int(v: &PropertyValue) -> Option<i64> {
    match v {
        PropertyValue::Integer16(n) => Some(i64::from(*n)),
        PropertyValue::Integer32(n) => Some(i64::from(*n)),
        PropertyValue::Integer64(n) | PropertyValue::Time(n) => Some(*n),
        PropertyValue::Boolean(b) => Some(i64::from(*b)),
        _ => None,
    }
}

impl Props for BTreeMap<u16, PropertyValue> {
    fn text(&self, id: u16) -> Option<String> {
        self.get(&id).and_then(value_text)
    }
    fn bytes(&self, id: u16) -> Option<Vec<u8>> {
        self.get(&id).and_then(value_bytes)
    }
    fn int(&self, id: u16) -> Option<i64> {
        self.get(&id).and_then(value_int)
    }
}

/// FILETIME (100 ns desde 1601) para segundos desde 1970.
fn filetime(value: i64) -> Option<i64> {
    let secs = value / 10_000_000 - 11_644_473_600;
    (secs > 0).then_some(secs)
}

/// Nome do charset de um code page do Windows, para o leitor de MIME decodificar.
fn charset_of(cpid: Option<i64>) -> String {
    match cpid {
        Some(65001) | None => "utf-8".into(),
        Some(20127) => "us-ascii".into(),
        Some(28591) => "iso-8859-1".into(),
        Some(n @ 28592..=28599) => format!("iso-8859-{}", n - 28590),
        Some(28605) => "iso-8859-15".into(),
        Some(n @ 1250..=1258) => format!("windows-{n}"),
        Some(932) => "shift_jis".into(),
        Some(936) => "gb2312".into(),
        Some(949) => "ks_c_5601-1987".into(),
        Some(950) => "big5".into(),
        Some(20866) => "koi8-r".into(),
        Some(_) => "windows-1252".into(),
    }
}

/// Monta a mensagem a partir das propriedades. `recipients` e `attachments` vêm das tabelas.
pub fn from_props(p: &impl Props, recipients: Vec<(i64, String, String)>, attachments: Vec<MapiAttachment>) -> MapiMessage {
    let from_address = p.text(SENDER_SMTP).or_else(|| p.text(SENT_REPRESENTING_SMTP)).or_else(|| p.text(SENDER_EMAIL).filter(|a| a.contains('@'))).or_else(|| p.text(SENT_REPRESENTING_EMAIL).filter(|a| a.contains('@'))).unwrap_or_default();
    let html = p.bytes(HTML).map(|b| {
        let b = if b.ends_with(&[0]) { b[..b.len() - 1].to_vec() } else { b };
        (b, charset_of(p.int(INTERNET_CPID)))
    });
    let (mut to, mut cc) = (Vec::new(), Vec::new());
    for (kind, name, address) in recipients {
        match kind {
            1 => to.push((name, address)),
            2 => cc.push((name, address)),
            _ => {}
        }
    }
    MapiMessage {
        subject: p.text(SUBJECT).unwrap_or_default(),
        from_name: p.text(SENT_REPRESENTING_NAME).or_else(|| p.text(SENDER_NAME)).unwrap_or_default(),
        from_address,
        to,
        cc,
        date: p.int(DELIVERY_TIME).or_else(|| p.int(CLIENT_SUBMIT_TIME)).and_then(filetime),
        transport_headers: p.text(TRANSPORT_HEADERS),
        message_id: p.text(INTERNET_MESSAGE_ID),
        in_reply_to: p.text(IN_REPLY_TO),
        references: p.text(INTERNET_REFERENCES),
        text: p.text(BODY),
        html,
        attachments,
        read: p.int(MESSAGE_FLAGS).is_none_or(|f| f & MSGFLAG_READ != 0),
        flagged: p.int(FLAG_STATUS) == Some(FLAG_STATUS_FLAGGED),
    }
}

/// Endereço de uma pessoa: o SMTP; um endereço do Exchange (`/O=…`) não serve.
fn recipient(p: &impl Props) -> Option<(i64, String, String)> {
    let kind = p.int(RECIPIENT_TYPE)?;
    let address = p.text(SMTP_ADDRESS).or_else(|| p.text(EMAIL_ADDRESS).filter(|a| a.contains('@'))).unwrap_or_default();
    let name = p.text(DISPLAY_NAME).unwrap_or_default();
    Some((kind, name, address))
}

fn attachment_meta(p: &impl Props) -> (String, String, Option<String>) {
    let name = p.text(ATTACH_LONG_FILENAME).or_else(|| p.text(ATTACH_FILENAME)).or_else(|| p.text(DISPLAY_NAME)).unwrap_or_else(|| "anexo".into());
    let mime = p.text(ATTACH_MIME_TAG).filter(|m| m.contains('/') && m.is_ascii()).unwrap_or_else(|| "application/octet-stream".into());
    (name, mime, p.text(ATTACH_CONTENT_ID))
}

/// Só mensagens de e-mail (IPM.Note e as variantes cifradas e assinadas); calendário, contatos,
/// tarefas e notas ficam de fora.
pub fn is_mail(p: &impl Props) -> bool {
    p.text(MESSAGE_CLASS).is_none_or(|c| {
        let c = c.to_ascii_lowercase();
        c.starts_with("ipm.note") || c == "ipm" || c.starts_with("report.ipm.note")
    })
}

// ---------------------------------------------------------------- MIME

/// Texto de cabeçalho: ASCII fica; o resto vira encoded-word. Sem quebras de linha.
fn header_text(text: &str) -> String {
    let clean: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    if clean.is_ascii() {
        clean
    } else {
        format!("=?utf-8?b?{}?=", STANDARD.encode(clean.as_bytes()))
    }
}

fn mailbox(name: &str, address: &str) -> String {
    let address: String = address.chars().filter(|c| !c.is_control() && !matches!(c, '<' | '>' | ',' | ' ')).collect();
    match (name.trim().is_empty(), address.is_empty()) {
        (true, true) => String::new(),
        (true, false) => format!("<{address}>"),
        (false, true) => format!("\"{}\" <>", header_text(name).replace('"', "'")),
        (false, false) => format!("\"{}\" <{address}>", header_text(name).replace('"', "'")),
    }
}

fn base64_lines(data: &[u8]) -> String {
    let encoded = STANDARD.encode(data);
    let mut out = String::with_capacity(encoded.len() + encoded.len() / 76 * 2);
    for chunk in encoded.as_bytes().chunks(76) {
        out.push_str(std::str::from_utf8(chunk).unwrap_or_default());
        out.push_str("\r\n");
    }
    out
}

/// Os cabeçalhos originais da internet, sem os de MIME (o corpo é remontado aqui).
fn kept_headers(headers: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in headers.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            break;
        }
        if !line.starts_with([' ', '\t']) {
            let name = line.split(':').next().unwrap_or_default().trim().to_ascii_lowercase();
            skipping = name.starts_with("content-") || name == "mime-version" || !line.contains(':');
        }
        if !skipping {
            out.push_str(line);
            out.push_str("\r\n");
        }
    }
    out
}

fn boundary(seed: &str, depth: usize) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("{seed}/{depth}").as_bytes());
    format!("epistula-{}", digest[..12].iter().map(|b| format!("{b:02x}")).collect::<String>())
}

/// A mensagem em MIME, pronta para o mesmo caminho de qualquer outra (leitura, busca, conversas).
pub fn to_mime(m: &MapiMessage) -> Vec<u8> {
    let mut head = String::new();
    match m.transport_headers.as_deref().map(kept_headers).filter(|h| h.to_ascii_lowercase().contains("from:")) {
        Some(original) => head.push_str(&original),
        None => {
            head.push_str(&format!("From: {}\r\n", mailbox(&m.from_name, &m.from_address)));
            if !m.to.is_empty() {
                head.push_str(&format!("To: {}\r\n", m.to.iter().map(|(n, a)| mailbox(n, a)).collect::<Vec<_>>().join(", ")));
            }
            if !m.cc.is_empty() {
                head.push_str(&format!("Cc: {}\r\n", m.cc.iter().map(|(n, a)| mailbox(n, a)).collect::<Vec<_>>().join(", ")));
            }
            head.push_str(&format!("Subject: {}\r\n", header_text(&m.subject)));
            if let Some(date) = m.date.and_then(|d| chrono::DateTime::from_timestamp(d, 0)) {
                head.push_str(&format!("Date: {}\r\n", date.to_rfc2822()));
            }
            for (name, value) in [("Message-ID", &m.message_id), ("In-Reply-To", &m.in_reply_to), ("References", &m.references)] {
                if let Some(v) = value {
                    head.push_str(&format!("{name}: {}\r\n", header_text(v)));
                }
            }
        }
    }
    head.push_str("MIME-Version: 1.0\r\n");
    let seed = format!("{}{}{:?}", m.subject, m.from_address, m.date);
    let body = body_part(m, &seed);
    let mut out = head.into_bytes();
    out.extend_from_slice(&body);
    out
}

/// Cabeçalho Content-Type e corpo: texto, HTML ou os dois, e os anexos.
fn body_part(m: &MapiMessage, seed: &str) -> Vec<u8> {
    let text = m.text.as_deref().map(|t| format!("Content-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{}", base64_lines(t.as_bytes())));
    let html = m.html.as_ref().map(|(b, cs)| format!("Content-Type: text/html; charset=\"{cs}\"\r\nContent-Transfer-Encoding: base64\r\n\r\n{}", base64_lines(b)));
    let readable = match (text, html) {
        (Some(t), Some(h)) => {
            let b = boundary(seed, 1);
            format!("Content-Type: multipart/alternative; boundary=\"{b}\"\r\n\r\n--{b}\r\n{t}--{b}\r\n{h}--{b}--\r\n")
        }
        (Some(t), None) => t,
        (None, Some(h)) => h,
        (None, None) => "Content-Type: text/plain; charset=utf-8\r\n\r\n".to_owned(),
    };
    if m.attachments.is_empty() {
        return readable.into_bytes();
    }
    let b = boundary(seed, 0);
    let mut out = format!("Content-Type: multipart/mixed; boundary=\"{b}\"\r\n\r\n--{b}\r\n{readable}").into_bytes();
    for a in &m.attachments {
        out.extend_from_slice(format!("\r\n--{b}\r\n").as_bytes());
        let name = header_text(&a.name).replace('"', "'");
        match &a.data {
            AttachmentBody::Bytes(data) => {
                let mut part = format!("Content-Type: {}; name=\"{name}\"\r\nContent-Disposition: attachment; filename=\"{name}\"\r\nContent-Transfer-Encoding: base64\r\n", a.mime);
                if let Some(cid) = &a.content_id {
                    part.push_str(&format!("Content-ID: <{}>\r\n", header_text(cid.trim_matches(['<', '>']))));
                }
                part.push_str("\r\n");
                part.push_str(&base64_lines(data));
                out.extend_from_slice(part.as_bytes());
            }
            AttachmentBody::Message(raw) => {
                out.extend_from_slice(format!("Content-Type: message/rfc822\r\nContent-Disposition: attachment; filename=\"{name}.eml\"\r\n\r\n").as_bytes());
                out.extend_from_slice(raw);
            }
        }
    }
    out.extend_from_slice(format!("\r\n--{b}--\r\n").as_bytes());
    out
}

pub fn flags_of(m: &MapiMessage) -> MessageFlags {
    MessageFlags { seen: m.read, flagged: m.flagged }
}

// ---------------------------------------------------------------- .pst

/// Uma pasta de e-mail do `.pst`: o caminho na árvore e o nó, para abrir de novo na leitura.
pub struct PstFolder {
    pub key: String,
    pub role: Role,
    pub count: u32,
    node: NodeId,
}

fn table_rows(table: &Rc<dyn TableContext>) -> Vec<(NodeId, BTreeMap<u16, PropertyValue>)> {
    let context = table.context();
    table
        .rows_matrix()
        .filter_map(|row| {
            let values = row.columns(context).ok()?;
            let mut props = BTreeMap::new();
            for (column, value) in context.columns().iter().zip(values) {
                if let Some(value) = value {
                    if let Ok(v) = table.read_column(&value, column.prop_type()) {
                        props.insert(column.prop_id(), v);
                    }
                }
            }
            Some((NodeId::from(u32::from(row.id())), props))
        })
        .collect()
}

fn folder_props(folder: &Rc<dyn Folder>) -> BTreeMap<u16, PropertyValue> {
    folder.properties().iter().map(|(k, v)| (*k, v.clone())).collect()
}

fn is_mail_folder(props: &BTreeMap<u16, PropertyValue>) -> bool {
    props.text(CONTAINER_CLASS).is_none_or(|c| c.to_ascii_lowercase().starts_with("ipf.note"))
}

/// As pastas de e-mail do `.pst`, na ordem da árvore (subpastas depois da mãe).
fn pst_folders(store: &Rc<dyn Store>) -> Result<Vec<PstFolder>, AppError> {
    let root_id = store.properties().ipm_sub_tree_entry_id().map_err(|_| AppError::ImportUnrecognized)?;
    let root = store.open_folder(&root_id).map_err(|_| AppError::ImportUnrecognized)?;
    let mut out = Vec::new();
    let mut stack: Vec<(Rc<dyn Folder>, String, usize)> = vec![(root, String::new(), 0)];
    while let Some((folder, prefix, depth)) = stack.pop() {
        let Some(table) = folder.hierarchy_table() else { continue };
        let mut children = Vec::new();
        for (node, _) in table_rows(table) {
            let Ok(entry) = store.properties().make_entry_id(node) else { continue };
            let Ok(child) = store.open_folder(&entry) else { continue };
            let props = folder_props(&child);
            let name = props.text(DISPLAY_NAME).unwrap_or_else(|| "Pasta".into()).replace('/', "-");
            let key = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
            if is_mail_folder(&props) {
                let count = child.contents_table().map_or(0, |t| t.rows_matrix().count()) as u32;
                out.push(PstFolder { role: role_for(&key), key: key.clone(), count, node });
            }
            if depth < 12 {
                children.push((child, key, depth + 1));
            }
        }
        stack.extend(children.into_iter().rev());
    }
    Ok(out)
}

fn open_store(path: &Path) -> Result<Rc<dyn Store>, AppError> {
    outlook_pst::open_store(path).map_err(|_| AppError::ImportUnrecognized)
}

/// Pastas e quantidades de um `.pst`, para o passo 2.
pub fn scan_pst(path: &Path) -> Result<Vec<PstFolder>, AppError> {
    let store = open_store(path)?;
    pst_folders(&store)
}

/// Mensagem anexada a outra no `.pst` (sem os anexos dela: o tipo genérico não os abre).
fn dyn_message(message: &Rc<dyn Message>) -> MapiMessage {
    let props: BTreeMap<u16, PropertyValue> = message.properties().iter().map(|(k, v)| (*k, v.clone())).collect();
    let recipients = message.recipient_table().map(|t| table_rows(t).into_iter().filter_map(|(_, p)| recipient(&p)).collect()).unwrap_or_default();
    from_props(&props, recipients, Vec::new())
}

/// Lê o `.pst` com os tipos concretos (Unicode ou ANSI), que é o que dá acesso aos anexos.
macro_rules! read_pst_with {
    ($file:ty, $store:ty, $message:ty, $attachment:ty, $path:expr, $wanted:expr, $cancel:expr, $on:expr) => {{
        let file = <$file>::open($path).map_err(|_| AppError::ImportUnrecognized)?;
        let typed: Rc<$store> = <$store>::read(Rc::new(file)).map_err(|_| AppError::ImportUnrecognized)?;
        let store: Rc<dyn Store> = typed.clone();
        for folder in pst_folders(&store)?.into_iter().filter(|f| $wanted.contains(&f.key)) {
            let Ok(entry) = store.properties().make_entry_id(folder.node) else { continue };
            let Ok(opened) = store.open_folder(&entry) else { continue };
            let Some(contents) = opened.contents_table() else { continue };
            for (node, _) in table_rows(contents) {
                if $cancel.load(Ordering::Relaxed) {
                    return Ok(());
                }
                let item = (|| -> Result<Option<(Vec<u8>, MessageFlags)>, Failure> {
                    let entry: EntryId = store.properties().make_entry_id(node).map_err(|_| Failure::Unreadable)?;
                    let message = <$message>::read(typed.clone(), &entry, None).map_err(|_| Failure::Unreadable)?;
                    let props: BTreeMap<u16, PropertyValue> = message.properties().iter().map(|(k, v)| (*k, v.clone())).collect();
                    if !is_mail(&props) {
                        return Ok(None);
                    }
                    let recipients = message.recipient_table().map(|t| table_rows(t).into_iter().filter_map(|(_, p)| recipient(&p)).collect()).unwrap_or_default();
                    let mut attachments = Vec::new();
                    let mut total = 0usize;
                    if let Some(table) = message.attachment_table() {
                        for (sub, _) in table_rows(table) {
                            let Ok(att) = <$attachment>::read(message.clone(), sub, None) else { continue };
                            let props: BTreeMap<u16, PropertyValue> = att.properties().iter().map(|(k, v)| (*k, v.clone())).collect();
                            let (name, mime, content_id) = attachment_meta(&props);
                            let data = match att.data() {
                                Some(AttachmentData::Binary(b)) => AttachmentBody::Bytes(b.buffer().to_vec()),
                                Some(AttachmentData::Message(inner)) => AttachmentBody::Message(to_mime(&dyn_message(inner))),
                                None => continue,
                            };
                            total += match &data {
                                AttachmentBody::Bytes(b) | AttachmentBody::Message(b) => b.len(),
                            };
                            if total > MAX_MESSAGE_BYTES {
                                return Err(Failure::TooLarge);
                            }
                            attachments.push(MapiAttachment { name, mime, content_id, data });
                        }
                    }
                    let m = from_props(&props, recipients, attachments);
                    let raw = to_mime(&m);
                    if raw.len() > MAX_MESSAGE_BYTES {
                        return Err(Failure::TooLarge);
                    }
                    Ok(Some((raw, flags_of(&m))))
                })();
                let go_on = match item {
                    Ok(Some((raw, flags))) => $on(Ok((folder.key.clone(), raw, flags))),
                    Ok(None) => true,
                    Err(f) => $on(Err(f)),
                };
                if !go_on {
                    return Ok(());
                }
            }
        }
        Ok(())
    }};
}

/// Entrega as mensagens das pastas escolhidas: `(pasta, MIME, flags)` ou a falha de uma mensagem.
pub fn read_pst(
    path: &Path,
    wanted: &std::collections::HashSet<String>,
    cancel: &AtomicBool,
    mut on: impl FnMut(Result<(String, Vec<u8>, MessageFlags), Failure>) -> bool,
) -> Result<(), AppError> {
    let unicode = UnicodePstFile::open(path).is_ok();
    if unicode {
        read_pst_with!(UnicodePstFile, UnicodeStore, UnicodeMessage, UnicodeAttachment, path, wanted, cancel, on)
    } else {
        read_pst_with!(AnsiPstFile, AnsiStore, AnsiMessage, AnsiAttachment, path, wanted, cancel, on)
    }
}

// ---------------------------------------------------------------- .msg

/// Propriedades de uma pasta do `.msg` (`__substg1.0_XXXXYYYY` e o stream de propriedades fixas).
#[derive(Default)]
struct MsgProps {
    values: BTreeMap<u16, MsgValue>,
}

enum MsgValue {
    Text(String),
    Bytes(Vec<u8>),
    Int(i64),
}

impl Props for MsgProps {
    fn text(&self, id: u16) -> Option<String> {
        match self.values.get(&id)? {
            MsgValue::Text(t) => Some(t.trim_end_matches('\0').to_owned()).filter(|s| !s.is_empty()),
            _ => None,
        }
    }
    fn bytes(&self, id: u16) -> Option<Vec<u8>> {
        match self.values.get(&id)? {
            MsgValue::Bytes(b) => Some(b.clone()),
            MsgValue::Text(t) => Some(t.clone().into_bytes()),
            MsgValue::Int(_) => None,
        }
    }
    fn int(&self, id: u16) -> Option<i64> {
        match self.values.get(&id)? {
            MsgValue::Int(n) => Some(*n),
            _ => None,
        }
    }
}

fn utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16_lossy(&units)
}

fn child(storage: &str, name: &str) -> String {
    format!("{}/{name}", storage.trim_end_matches('/'))
}

fn read_storage_props<F: Read + std::io::Seek>(cf: &mut cfb::CompoundFile<F>, storage: &str, header: usize, budget: &mut usize) -> Result<MsgProps, Failure> {
    let mut props = MsgProps::default();
    let entries: Vec<(String, bool)> = cf.read_storage(storage).map_err(|_| Failure::Unreadable)?.map(|e| (e.name().to_owned(), e.is_stream())).collect();
    for (name, is_stream) in entries {
        let path = child(storage, &name);
        if is_stream && name.len() == 20 && name.starts_with("__substg1.0_") {
            let (Ok(id), Ok(kind)) = (u16::from_str_radix(&name[12..16], 16), u16::from_str_radix(&name[16..20], 16)) else { continue };
            let mut data = Vec::new();
            cf.open_stream(&path).map_err(|_| Failure::Unreadable)?.take((MAX_MESSAGE_BYTES + 1) as u64).read_to_end(&mut data).map_err(|_| Failure::Unreadable)?;
            *budget = budget.checked_sub(data.len()).ok_or(Failure::TooLarge)?;
            let value = match kind {
                0x001F => MsgValue::Text(utf16(&data)),
                0x001E => MsgValue::Text(String::from_utf8_lossy(&data).into_owned()),
                0x0102 => MsgValue::Bytes(data),
                _ => continue,
            };
            props.values.insert(id, value);
        } else if is_stream && name == "__properties_version1.0" {
            let mut data = Vec::new();
            cf.open_stream(&path).map_err(|_| Failure::Unreadable)?.take(1 << 20).read_to_end(&mut data).map_err(|_| Failure::Unreadable)?;
            for entry in data.get(header..).unwrap_or_default().chunks_exact(16) {
                let kind = u16::from_le_bytes([entry[0], entry[1]]);
                let id = u16::from_le_bytes([entry[2], entry[3]]);
                let value = &entry[8..16];
                let n = match kind {
                    0x0002 => i64::from(i16::from_le_bytes([value[0], value[1]])),
                    0x0003 => i64::from(i32::from_le_bytes([value[0], value[1], value[2], value[3]])),
                    0x000B => i64::from(value[0] != 0),
                    0x0014 | 0x0040 => i64::from_le_bytes(value.try_into().unwrap_or_default()),
                    _ => continue,
                };
                props.values.entry(id).or_insert(MsgValue::Int(n));
            }
        }
    }
    Ok(props)
}

/// Uma mensagem `.msg` (ou embutida num `.msg`, `root` com o caminho da pasta dela).
fn read_msg_storage<F: Read + std::io::Seek>(cf: &mut cfb::CompoundFile<F>, root: &str, top: bool, depth: usize, budget: &mut usize) -> Result<MapiMessage, Failure> {
    let props = read_storage_props(cf, root, if top { 32 } else { 24 }, budget)?;
    let children: Vec<String> = cf.read_storage(root).map_err(|_| Failure::Unreadable)?.filter(|e| !e.is_stream()).map(|e| e.name().to_owned()).collect();
    let mut recipients = Vec::new();
    let mut attachments = Vec::new();
    for name in children {
        let path = child(root, &name);
        if name.starts_with("__recip_version1.0_") {
            let p = read_storage_props(cf, &path, 8, budget)?;
            if let Some(r) = recipient(&p) {
                recipients.push(r);
            }
        } else if name.starts_with("__attach_version1.0_") {
            let p = read_storage_props(cf, &path, 8, budget)?;
            let (name, mime, content_id) = attachment_meta(&p);
            let embedded = format!("{path}/__substg1.0_3701000D");
            let data = if cf.is_storage(&embedded) && depth < MAX_EMBEDDED_DEPTH {
                AttachmentBody::Message(to_mime(&read_msg_storage(cf, &embedded, false, depth + 1, budget)?))
            } else if let Some(bytes) = p.bytes(0x3701) {
                AttachmentBody::Bytes(bytes)
            } else {
                continue;
            };
            attachments.push(MapiAttachment { name, mime, content_id, data });
        }
    }
    Ok(from_props(&props, recipients, attachments))
}

/// Lê um `.msg` e devolve a mensagem em MIME.
pub fn read_msg(path: &Path) -> Result<(Vec<u8>, MessageFlags), Failure> {
    let len = std::fs::metadata(path).map_err(|_| Failure::Unreadable)?.len() as usize;
    if len > MAX_MESSAGE_BYTES * 2 {
        return Err(Failure::TooLarge);
    }
    let mut cf = cfb::open(path).map_err(|_| Failure::Unreadable)?;
    let mut probe = MAX_MESSAGE_BYTES;
    if !is_mail(&read_storage_props(&mut cf, "/", 32, &mut probe)?) {
        return Err(Failure::Unreadable);
    }
    let mut budget = MAX_MESSAGE_BYTES;
    let m = read_msg_storage(&mut cf, "/", true, 0, &mut budget)?;
    Ok((to_mime(&m), flags_of(&m)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mail_parser::MimeHeaders;
    use std::io::Write;

    fn props(values: &[(u16, PropertyValueLike)]) -> MsgProps {
        let mut p = MsgProps::default();
        for (id, v) in values {
            p.values.insert(
                *id,
                match v {
                    PropertyValueLike::T(t) => MsgValue::Text((*t).to_owned()),
                    PropertyValueLike::I(n) => MsgValue::Int(*n),
                    PropertyValueLike::B(b) => MsgValue::Bytes(b.to_vec()),
                },
            );
        }
        p
    }

    enum PropertyValueLike {
        T(&'static str),
        I(i64),
        B(&'static [u8]),
    }
    use PropertyValueLike::{B, I, T};

    fn parse(raw: &[u8]) -> mail_parser::Message<'_> {
        mail_parser::MessageParser::default().parse(raw).unwrap()
    }

    #[test]
    fn mapi_properties_become_mime_that_reads_back() {
        // 1 jan 2024 10:00 UTC em FILETIME.
        let when = (1_704_103_200i64 + 11_644_473_600) * 10_000_000;
        let p = props(&[
            (SUBJECT, T("Relatório de março")),
            (SENT_REPRESENTING_NAME, T("Ana Lima")),
            (SENDER_EMAIL, T("/O=EMPRESA/OU=EXCHANGE/CN=ANA")),
            (SENDER_SMTP, T("ana@empresa.com")),
            (DELIVERY_TIME, I(when)),
            (BODY, T("Olá, segue o relatório.")),
            (HTML, B(b"<p>Ol\xe1, segue o <b>relat\xf3rio</b>.</p>")),
            (INTERNET_CPID, I(1252)),
            (INTERNET_MESSAGE_ID, T("<abc@empresa.com>")),
            (MESSAGE_FLAGS, I(0)),
            (FLAG_STATUS, I(2)),
        ]);
        let attachment = MapiAttachment { name: "relatório.pdf".into(), mime: "application/pdf".into(), content_id: None, data: AttachmentBody::Bytes(b"%PDF-1.4".to_vec()) };
        let m = from_props(&p, vec![(1, "Bia".into(), "bia@x.com".into()), (2, "Caio".into(), "caio@x.com".into()), (3, "Oculto".into(), "oculto@sigilo.com".into())], vec![attachment]);
        assert_eq!(flags_of(&m), MessageFlags { seen: false, flagged: true });
        let raw = to_mime(&m);
        assert!(raw.is_ascii(), "tudo em 7 bits");
        let parsed = parse(&raw);
        assert_eq!(parsed.subject(), Some("Relatório de março"));
        assert_eq!(parsed.from().and_then(|f| f.first()).and_then(|a| a.address()), Some("ana@empresa.com"));
        assert_eq!(parsed.to().and_then(|t| t.first()).and_then(|a| a.address()), Some("bia@x.com"));
        assert!(!String::from_utf8_lossy(&raw).contains("sigilo.com"), "Cco não vira cabeçalho");
        assert_eq!(parsed.message_id(), Some("abc@empresa.com"));
        assert_eq!(parsed.date().map(|d| d.to_timestamp()), Some(1_704_103_200));
        assert_eq!(parsed.body_text(0).as_deref(), Some("Olá, segue o relatório."));
        assert!(parsed.body_html(0).is_some_and(|h| h.contains("relatório")), "o HTML em windows-1252 é decodificado na leitura");
        assert_eq!(parsed.attachments().next().and_then(|a| a.attachment_name()), Some("relatório.pdf"));
    }

    #[test]
    fn transport_headers_are_kept_and_mime_is_rebuilt() {
        let p = props(&[
            (TRANSPORT_HEADERS, T("Received: from x\r\nFrom: Ana <ana@x.com>\r\nTo: bia@x.com\r\nSubject: Original\r\nMessage-ID: <orig@x>\r\nContent-Type: multipart/related; boundary=velho\r\nMIME-Version: 1.0\r\n\r\n")),
            (SUBJECT, T("Original")),
            (BODY, T("Corpo")),
        ]);
        let raw = to_mime(&from_props(&p, vec![], vec![]));
        let text = String::from_utf8_lossy(&raw);
        assert!(text.contains("Received: from x") && !text.contains("boundary=velho"));
        assert_eq!(parse(&raw).message_id(), Some("orig@x"));
        assert_eq!(parse(&raw).body_text(0).as_deref(), Some("Corpo"));
    }

    #[test]
    fn only_mail_classes_are_imported() {
        assert!(is_mail(&props(&[(MESSAGE_CLASS, T("IPM.Note"))])));
        assert!(is_mail(&props(&[(MESSAGE_CLASS, T("IPM.Note.SMIME"))])));
        assert!(!is_mail(&props(&[(MESSAGE_CLASS, T("IPM.Appointment"))])));
        assert!(!is_mail(&props(&[(MESSAGE_CLASS, T("IPM.Contact"))])));
        assert_eq!(charset_of(Some(28592)), "iso-8859-2");
    }

    fn stream<F: Read + Write + std::io::Seek>(cf: &mut cfb::CompoundFile<F>, path: &str, data: &[u8]) {
        cf.create_stream(path).unwrap().write_all(data).unwrap();
    }

    fn utf16_bytes(s: &str) -> Vec<u8> {
        s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
    }

    #[test]
    fn msg_files_are_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mensagem.msg");
        {
            let mut cf = cfb::create(&path).unwrap();
            stream(&mut cf, "/__substg1.0_001A001F", &utf16_bytes("IPM.Note"));
            stream(&mut cf, "/__substg1.0_0037001F", &utf16_bytes("Proposta comercial"));
            stream(&mut cf, "/__substg1.0_0042001F", &utf16_bytes("Ana Lima"));
            stream(&mut cf, "/__substg1.0_5D01001F", &utf16_bytes("ana@empresa.com"));
            stream(&mut cf, "/__substg1.0_1000001F", &utf16_bytes("Segue a proposta."));
            // Propriedades fixas: cabeçalho de 32 bytes e PR_MESSAGE_FLAGS = lida.
            let mut fixed = vec![0u8; 32];
            fixed.extend_from_slice(&[0x03, 0x00, 0x07, 0x0E, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]);
            stream(&mut cf, "/__properties_version1.0", &fixed);
            cf.create_storage("/__recip_version1.0_#00000000").unwrap();
            stream(&mut cf, "/__recip_version1.0_#00000000/__substg1.0_3001001F", &utf16_bytes("Bia"));
            stream(&mut cf, "/__recip_version1.0_#00000000/__substg1.0_39FE001F", &utf16_bytes("bia@x.com"));
            let mut recip = vec![0u8; 8];
            recip.extend_from_slice(&[0x03, 0x00, 0x15, 0x0C, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]);
            stream(&mut cf, "/__recip_version1.0_#00000000/__properties_version1.0", &recip);
            cf.create_storage("/__attach_version1.0_#00000000").unwrap();
            stream(&mut cf, "/__attach_version1.0_#00000000/__substg1.0_3707001F", &utf16_bytes("tabela.csv"));
            stream(&mut cf, "/__attach_version1.0_#00000000/__substg1.0_370E001F", &utf16_bytes("text/csv"));
            stream(&mut cf, "/__attach_version1.0_#00000000/__substg1.0_37010102", b"a;b\n1;2\n");
            cf.flush().unwrap();
        }
        let (raw, flags) = read_msg(&path).unwrap();
        assert!(flags.seen);
        let parsed = parse(&raw);
        assert_eq!(parsed.subject(), Some("Proposta comercial"));
        assert_eq!(parsed.from().and_then(|f| f.first()).and_then(|a| a.name()), Some("Ana Lima"));
        assert_eq!(parsed.to().and_then(|t| t.first()).and_then(|a| a.address()), Some("bia@x.com"));
        let att = parsed.attachments().next().unwrap();
        assert_eq!((att.attachment_name(), att.contents()), (Some("tabela.csv"), b"a;b\n1;2\n".as_slice()));
    }

    #[test]
    fn empty_pst_opens_with_no_mail_folders_to_fill() {
        let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/empty.pst"));
        let folders = scan_pst(path).unwrap();
        assert!(folders.iter().all(|f| f.count == 0));
        let mut seen = 0;
        read_pst(path, &folders.iter().map(|f| f.key.clone()).collect(), &AtomicBool::new(false), |_| {
            seen += 1;
            true
        })
        .unwrap();
        assert_eq!(seen, 0);
    }
}
