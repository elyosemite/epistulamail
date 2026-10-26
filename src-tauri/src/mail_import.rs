//! Leitura de arquivos de e-mail de outros programas (US15): reconhece o formato pelo conteúdo,
//! descobre as pastas e entrega as mensagens uma a uma, em streaming (um Takeout passa fácil de
//! dezenas de GB). Só lê: nada é escrito no disco, nada vai para a rede e nada do conteúdo, dos
//! nomes de pastas ou do caminho vai para o log. Gravar no banco fica com `import.rs`.

use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use mail_parser::MessageParser;
use serde::Serialize;

use crate::error::AppError;
use crate::mailbox::Role;
use crate::message::MessageFlags;

/// Uma mensagem maior que isso falha sozinha ("grande demais"): protege o app de arquivos malformados.
pub const MAX_MESSAGE_BYTES: usize = 50 * 1024 * 1024;
const MAX_DEPTH: usize = 12;
/// Itens de pasta lidos ao procurar mensagens, para uma pasta errada (o disco inteiro) não travar o app.
const MAX_ENTRIES: usize = 500_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Takeout,
    Mbox,
    Thunderbird,
    AppleMail,
    Maildir,
    Eml,
    Emlx,
    Pst,
    Msg,
}

impl Format {
    pub fn as_str(self) -> &'static str {
        match self {
            Format::Takeout => "takeout",
            Format::Mbox => "mbox",
            Format::Thunderbird => "thunderbird",
            Format::AppleMail => "apple_mail",
            Format::Maildir => "maildir",
            Format::Eml => "eml",
            Format::Emlx => "emlx",
            Format::Pst => "pst",
            Format::Msg => "msg",
        }
    }

    pub fn parse(value: &str) -> Option<Format> {
        [Format::Takeout, Format::Mbox, Format::Thunderbird, Format::AppleMail, Format::Maildir, Format::Eml, Format::Emlx, Format::Pst, Format::Msg]
            .into_iter()
            .find(|f| f.as_str() == value)
    }
}

/// Uma pasta encontrada no arquivo, para o passo 2 (CA2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceFolder {
    /// Identifica a pasta dentro do arquivo (caminho relativo, ou o marcador no Takeout).
    pub key: String,
    pub name: String,
    pub role: Role,
    pub depth: u32,
    pub count: u32,
    pub bytes: u64,
    /// Marcador do Takeout (a tela agrupa numa linha só, D4).
    pub label: bool,
    /// Começa marcada: tudo menos Spam e Lixeira.
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scan {
    pub format: Format,
    /// Nome do arquivo ou da pasta escolhida.
    pub name: String,
    pub bytes: u64,
    /// Mensagens no arquivo (sem repetir as que estão em vários marcadores).
    pub estimate: u32,
    pub folders: Vec<SourceFolder>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    TooLarge,
    Unreadable,
}

impl Failure {
    pub fn as_str(self) -> &'static str {
        match self {
            Failure::TooLarge => "too_large",
            Failure::Unreadable => "unreadable",
        }
    }
}

/// O que a leitura entrega: uma mensagem com as pastas de destino, ou uma falha (sem conteúdo).
#[derive(Debug)]
pub enum Item {
    Message { folders: Vec<String>, raw: Vec<u8>, flags: MessageFlags },
    Failed(Failure),
}

// ---------------------------------------------------------------- nomes e papéis

/// Papel de uma pasta pelo nome, em inglês e em português (Outlook, Thunderbird, Apple Mail).
pub fn role_for(name: &str) -> Role {
    let leaf = name.rsplit('/').next().unwrap_or(name).trim().to_lowercase();
    match leaf.as_str() {
        "inbox" | "caixa de entrada" | "entrada" => Role::Inbox,
        "sent" | "sent items" | "sent messages" | "sent mail" | "enviados" | "itens enviados" | "mensagens enviadas" | "e-mails enviados" => Role::Sent,
        "drafts" | "draft" | "rascunhos" | "rascunho" => Role::Drafts,
        "trash" | "deleted items" | "deleted messages" | "bin" | "lixeira" | "itens excluídos" | "itens excluidos" | "mensagens apagadas" => Role::Trash,
        "junk" | "spam" | "junk e-mail" | "junk email" | "bulk mail" | "lixo eletrônico" | "lixo eletronico" | "lixo" => Role::Spam,
        _ => Role::Custom,
    }
}

fn starts_selected(role: Role) -> bool {
    !matches!(role, Role::Spam | Role::Trash)
}

fn folder(key: String, name: String, role: Role, depth: u32) -> SourceFolder {
    SourceFolder { key, name, role, depth, count: 0, bytes: 0, label: false, selected: starts_selected(role) }
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}/{name}")
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn stem(name: &str, ext: &str) -> String {
    name.strip_suffix(ext).unwrap_or(name).to_owned()
}

fn depth_of(key: &str) -> u32 {
    key.matches('/').count() as u32
}

// ---------------------------------------------------------------- reconhecer

const OLE_MAGIC: &[u8] = &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

fn head_of(path: &Path, n: usize) -> Result<Vec<u8>, AppError> {
    let mut file = File::open(path).map_err(|_| AppError::ImportSourceMissing)?;
    let mut buf = vec![0u8; n];
    let mut read = 0;
    while read < n {
        match file.read(&mut buf[read..]) {
            Ok(0) => break,
            Ok(k) => read += k,
            Err(_) => return Err(AppError::ImportSourceMissing),
        }
    }
    buf.truncate(read);
    Ok(buf)
}

/// Parece uma mensagem RFC 822: começa com cabeçalhos e tem algum dos que toda mensagem tem.
pub fn looks_like_message(head: &[u8]) -> bool {
    let text = String::from_utf8_lossy(head);
    let mut known = false;
    for (i, line) in text.split('\n').take(80).enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            return known && i > 0;
        }
        if line.starts_with([' ', '\t']) {
            if i == 0 {
                return false;
            }
            continue;
        }
        let Some((name, _)) = line.split_once(':') else { return false };
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_graphic() && b != b':') {
            return false;
        }
        known |= matches!(name.to_ascii_lowercase().as_str(), "from" | "date" | "subject" | "message-id" | "received" | "return-path" | "mime-version" | "to");
    }
    known
}

fn is_mbox_file(path: &Path) -> bool {
    head_of(path, 5).is_ok_and(|h| h == b"From ")
}

/// `.emlx` do Apple Mail: a primeira linha é o tamanho da mensagem em bytes.
fn emlx_length(head: &[u8]) -> Option<usize> {
    let end = head.iter().position(|&b| b == b'\n')?;
    let digits = std::str::from_utf8(&head[..end]).ok()?.trim();
    if digits.is_empty() || digits.len() > 12 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

fn is_maildir(dir: &Path) -> bool {
    dir.join("cur").is_dir() && dir.join("new").is_dir()
}

/// Formato de um arquivo solto, pelos primeiros bytes.
fn detect_file(path: &Path) -> Result<Format, AppError> {
    let head = head_of(path, 64 * 1024)?;
    if head.starts_with(b"!BDN") {
        return Ok(Format::Pst);
    }
    if head.starts_with(OLE_MAGIC) {
        return Ok(Format::Msg);
    }
    if head.starts_with(b"From ") {
        let body = head.iter().position(|&b| b == b'\n').map_or(&head[..0], |i| &head[i + 1..]);
        return Ok(if header_value(body, "x-gmail-labels").is_some() { Format::Takeout } else { Format::Mbox });
    }
    if let Some(len) = emlx_length(&head) {
        let start = head.iter().position(|&b| b == b'\n').unwrap_or(0) + 1;
        if len > 0 && looks_like_message(&head[start..]) {
            return Ok(Format::Emlx);
        }
    }
    if looks_like_message(&head) {
        return Ok(Format::Eml);
    }
    Err(AppError::ImportUnrecognized)
}

// ---------------------------------------------------------------- onde estão as mensagens

#[derive(Debug, Clone)]
enum Place {
    Mbox(PathBuf),
    Maildir(PathBuf),
    Files(Vec<PathBuf>),
}

#[derive(Debug, Clone)]
struct Located {
    folder: SourceFolder,
    place: Place,
}

#[derive(Default)]
struct Kinds {
    thunderbird: bool,
    apple: bool,
    maildir: bool,
    mbox: bool,
    eml: bool,
    msg: bool,
}

struct Walk {
    found: Vec<Located>,
    kinds: Kinds,
    budget: usize,
}

impl Walk {
    fn push(&mut self, name: String, place: Place) {
        let role = role_for(&name);
        self.push_as(name, place, role);
    }

    fn push_as(&mut self, name: String, place: Place, role: Role) {
        let mut key = name.clone();
        let mut n = 2;
        while self.found.iter().any(|l| l.folder.key == key) {
            key = format!("{name} ({n})");
            n += 1;
        }
        let depth = depth_of(&key);
        self.found.push(Located { folder: folder(key.clone(), key, role, depth), place });
    }

    fn entries(&mut self, dir: &Path) -> Vec<PathBuf> {
        let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
        let mut out: Vec<PathBuf> = Vec::new();
        for entry in read.flatten() {
            if self.budget == 0 {
                break;
            }
            self.budget -= 1;
            out.push(entry.path());
        }
        out.sort();
        out
    }

    fn visit(&mut self, dir: &Path, prefix: &str, depth: usize) {
        if depth > MAX_DEPTH || self.budget == 0 {
            return;
        }
        if is_maildir(dir) {
            self.kinds.maildir = true;
            // A Maildir escolhida é a Caixa de entrada; as de dentro de outra pasta ficam com o nome.
            let own = if prefix.is_empty() { file_name(dir) } else { prefix.to_owned() };
            let role = if prefix.is_empty() { Role::Inbox } else { role_for(&own) };
            self.push_as(own.clone(), Place::Maildir(dir.to_path_buf()), role);
            // Maildir++: subpastas ".Trabalho.Clientes" ao lado de cur/new/tmp.
            for child in self.entries(dir) {
                let name = file_name(&child);
                if child.is_dir() && name.len() > 1 && name.starts_with('.') && is_maildir(&child) {
                    let sub = name.trim_start_matches('.').replace('.', "/");
                    self.push(join(&own, &sub), Place::Maildir(child));
                }
            }
            return;
        }
        let mut emls = Vec::new();
        let mut emlxs = Vec::new();
        let mut msgs = Vec::new();
        for path in self.entries(dir) {
            let name = file_name(&path);
            if path.is_dir() {
                if let Some(base) = name.strip_suffix(".sbd") {
                    self.kinds.thunderbird = true;
                    self.visit(&path, &join(prefix, base), depth + 1);
                } else if name.ends_with(".mbox") {
                    self.kinds.apple = true;
                    self.apple_bundle(&path, &join(prefix, &stem(&name, ".mbox")), depth + 1);
                } else {
                    self.visit(&path, &join(prefix, &name), depth + 1);
                }
                continue;
            }
            let lower = name.to_lowercase();
            if lower.ends_with(".msf") {
                self.kinds.thunderbird = true;
            } else if lower.ends_with(".emlx") {
                emlxs.push(path);
            } else if lower.ends_with(".eml") {
                emls.push(path);
            } else if lower.ends_with(".msg") && head_of(&path, 8).is_ok_and(|h| h == OLE_MAGIC) {
                msgs.push(path);
            } else if is_mbox_file(&path) {
                self.kinds.mbox = true;
                self.push(join(prefix, &name), Place::Mbox(path));
            }
        }
        let here = if prefix.is_empty() { file_name(dir) } else { prefix.to_owned() };
        if !emls.is_empty() {
            self.kinds.eml = true;
            self.push(here.clone(), Place::Files(emls));
        }
        if !emlxs.is_empty() {
            self.kinds.apple = true;
            self.push(here.clone(), Place::Files(emlxs));
        }
        if !msgs.is_empty() {
            self.kinds.msg = true;
            self.push(here, Place::Files(msgs));
        }
    }

    /// Pacote `.mbox` do Apple Mail: o arquivo `mbox` (exportado) ou as `.emlx` da biblioteca, e as
    /// caixas filhas (outros `.mbox` dentro dele).
    fn apple_bundle(&mut self, dir: &Path, name: &str, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        let exported = dir.join("mbox");
        if exported.is_file() && is_mbox_file(&exported) {
            self.push(name.to_owned(), Place::Mbox(exported));
        } else {
            let mut files = Vec::new();
            self.collect_emlx(dir, depth, &mut files);
            if !files.is_empty() {
                files.sort();
                self.push(name.to_owned(), Place::Files(files));
            }
        }
        for child in self.entries(dir) {
            let child_name = file_name(&child);
            if child.is_dir() && child_name.ends_with(".mbox") {
                self.apple_bundle(&child, &join(name, &stem(&child_name, ".mbox")), depth + 1);
            }
        }
    }

    fn collect_emlx(&mut self, dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        if depth > MAX_DEPTH {
            return;
        }
        for path in self.entries(dir) {
            let name = file_name(&path);
            if path.is_dir() {
                if !name.ends_with(".mbox") {
                    self.collect_emlx(&path, depth + 1, out);
                }
            } else if name.to_lowercase().ends_with(".emlx") {
                out.push(path);
            }
        }
    }
}

/// Formato e pastas de um arquivo ou de uma pasta.
fn locate(path: &Path) -> Result<(Format, Vec<Located>), AppError> {
    if path.is_dir() {
        let mut walk = Walk { found: Vec::new(), kinds: Kinds::default(), budget: MAX_ENTRIES };
        walk.visit(path, "", 0);
        if walk.found.is_empty() {
            return Err(AppError::ImportUnrecognized);
        }
        let k = &walk.kinds;
        let format = if k.thunderbird {
            Format::Thunderbird
        } else if k.apple {
            Format::AppleMail
        } else if k.maildir {
            Format::Maildir
        } else if k.mbox {
            Format::Mbox
        } else if k.msg && !k.eml {
            Format::Msg
        } else {
            Format::Eml
        };
        return Ok((format, walk.found));
    }
    if !path.is_file() {
        return Err(AppError::ImportSourceMissing);
    }
    let format = detect_file(path)?;
    let name = file_name(path);
    let single = |place: Place, name: String| {
        let role = match role_for(&name) {
            Role::Custom => Role::Inbox,
            r => r,
        };
        vec![Located { folder: folder(name.clone(), name, role, 0), place }]
    };
    let found = match format {
        Format::Mbox => {
            // O arquivo `mbox` de dentro de um pacote do Apple Mail tem o nome no pacote.
            let shown = if name == "mbox" { path.parent().map(|p| stem(&file_name(p), ".mbox")).unwrap_or(name) } else { stem(&name, ".mbox") };
            single(Place::Mbox(path.to_path_buf()), shown)
        }
        Format::Eml | Format::Emlx | Format::Msg => single(Place::Files(vec![path.to_path_buf()]), "Mensagens".to_owned()),
        // O Takeout tem as pastas nos marcadores; o `.pst`, na árvore dele (`mapi_import`).
        Format::Takeout | Format::Pst => Vec::new(),
        Format::Thunderbird | Format::AppleMail | Format::Maildir => return Err(AppError::ImportUnrecognized),
    };
    Ok((format, found))
}

// ---------------------------------------------------------------- examinar (passo 2)

/// Examina o arquivo ou a pasta: formato, pastas, quantidade e tamanho. Lê o arquivo inteiro para
/// contar (um mbox não tem índice), então pode levar alguns segundos por GB.
pub fn scan(path: &Path, cancel: &AtomicBool) -> Result<Scan, AppError> {
    let (format, located) = locate(path)?;
    let name = file_name(path);
    let bytes_of = |p: &Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    if format == Format::Takeout {
        return scan_takeout(path, name, bytes_of(path), cancel);
    }
    if format == Format::Pst {
        let file_bytes = bytes_of(path);
        let found = crate::mapi_import::scan_pst(path)?;
        let total: u32 = found.iter().map(|f| f.count).sum();
        // O `.pst` não diz o tamanho por pasta sem abrir cada mensagem: a proporção do arquivo basta.
        let folders = found
            .into_iter()
            .map(|f| {
                let depth = depth_of(&f.key);
                let bytes = if total == 0 { 0 } else { file_bytes * u64::from(f.count) / u64::from(total) };
                SourceFolder { count: f.count, bytes, ..folder(f.key.clone(), f.key, f.role, depth) }
            })
            .collect();
        return Ok(Scan { format, name, bytes: file_bytes, estimate: total, folders });
    }
    let mut folders = Vec::new();
    let mut total_bytes = 0;
    let mut estimate = 0u32;
    for l in located {
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::InvalidInput);
        }
        let mut f = l.folder;
        match &l.place {
            Place::Mbox(p) => {
                f.bytes = bytes_of(p);
                each_mbox(p, cancel, |_| {
                    f.count += 1;
                    true
                })?;
            }
            Place::Maildir(dir) => {
                for sub in ["cur", "new"] {
                    for entry in std::fs::read_dir(dir.join(sub)).into_iter().flatten().flatten() {
                        if entry.path().is_file() {
                            f.count += 1;
                            f.bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
                        }
                    }
                }
            }
            Place::Files(files) => {
                f.count = files.len() as u32;
                f.bytes = files.iter().map(|p| bytes_of(p)).sum();
            }
        }
        total_bytes += f.bytes;
        estimate = estimate.saturating_add(f.count);
        folders.push(f);
    }
    Ok(Scan { format, name, bytes: total_bytes, estimate, folders })
}

// ---------------------------------------------------------------- Google Takeout (D4)

const TAKEOUT_ALL: &str = "all";

/// Marcadores do Takeout que são pastas do sistema; os que viram flags; os que não interessam.
enum TakeoutLabel {
    Folder(&'static str, Role),
    Unread,
    Starred,
    Ignored,
    User(String),
}

fn takeout_label(label: &str) -> TakeoutLabel {
    let lower = label.trim().to_lowercase();
    match lower.as_str() {
        "inbox" | "caixa de entrada" => TakeoutLabel::Folder("inbox", Role::Inbox),
        "sent" | "enviados" | "e-mails enviados" | "enviado" => TakeoutLabel::Folder("sent", Role::Sent),
        "drafts" | "rascunhos" | "rascunho" => TakeoutLabel::Folder("drafts", Role::Drafts),
        "spam" => TakeoutLabel::Folder("spam", Role::Spam),
        "trash" | "lixeira" => TakeoutLabel::Folder("trash", Role::Trash),
        "unread" | "não lidos" | "não lido" | "nao lidos" => TakeoutLabel::Unread,
        "starred" | "com estrela" => TakeoutLabel::Starred,
        "important" | "importante" | "opened" | "aberto" | "abertos" | "archived" | "arquivados" | "arquivado" => TakeoutLabel::Ignored,
        _ if lower.starts_with("category ") || lower.starts_with("categoria") => TakeoutLabel::Ignored,
        _ if lower.is_empty() => TakeoutLabel::Ignored,
        _ => TakeoutLabel::User(label.trim().to_owned()),
    }
}

/// `X-Gmail-Labels: Inbox,"Projetos, 2024",Importante` em lista (vírgulas entre aspas ficam).
pub fn split_labels(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for c in value.chars() {
        match c {
            '"' => quoted = !quoted,
            ',' if !quoted => {
                out.push(std::mem::take(&mut current).trim().to_owned());
            }
            _ => current.push(c),
        }
    }
    out.push(current.trim().to_owned());
    out.retain(|l| !l.is_empty());
    out
}

/// Pastas de destino e flags de uma mensagem do Takeout.
fn takeout_targets(labels: &[String]) -> (Vec<String>, MessageFlags) {
    let mut keys: Vec<String> = Vec::new();
    let mut flags = MessageFlags { seen: true, flagged: false };
    let mut hidden = false;
    for label in labels {
        match takeout_label(label) {
            TakeoutLabel::Folder(key, role) => {
                hidden |= matches!(role, Role::Spam | Role::Trash);
                keys.push(key.to_owned());
            }
            TakeoutLabel::Unread => flags.seen = false,
            TakeoutLabel::Starred => flags.flagged = true,
            TakeoutLabel::Ignored => {}
            TakeoutLabel::User(name) => keys.push(format!("label:{name}")),
        }
    }
    // "Todos os e-mails", como no Gmail: tudo menos Spam e Lixeira.
    if !hidden {
        keys.push(TAKEOUT_ALL.to_owned());
    }
    keys.dedup();
    (keys, flags)
}

fn labels_of(raw: &[u8]) -> Vec<String> {
    header_value(raw, "x-gmail-labels").map(|v| split_labels(&decode_words(&v))).unwrap_or_default()
}

fn scan_takeout(path: &Path, name: String, bytes: u64, cancel: &AtomicBool) -> Result<Scan, AppError> {
    let mut counts: BTreeMap<String, (u32, u64)> = BTreeMap::new();
    let mut estimate = 0u32;
    each_mbox(path, cancel, |entry| {
        estimate += 1;
        if let MboxEntry::Message(raw) = &entry {
            let (keys, _) = takeout_targets(&labels_of(raw));
            for key in keys {
                let c = counts.entry(key).or_default();
                c.0 += 1;
                c.1 += raw.len() as u64;
            }
        }
        true
    })?;
    let mut folders = Vec::new();
    let system = [("inbox", Role::Inbox), ("sent", Role::Sent), ("drafts", Role::Drafts), (TAKEOUT_ALL, Role::All), ("spam", Role::Spam), ("trash", Role::Trash)];
    for (key, role) in system {
        let (count, size) = counts.remove(key).unwrap_or_default();
        if count > 0 || role == Role::Trash {
            let label = role.label().unwrap_or(key).to_owned();
            folders.push(SourceFolder { count, bytes: size, ..folder(key.to_owned(), label, role, 0) });
        }
    }
    for (key, (count, size)) in counts {
        let label = key.strip_prefix("label:").unwrap_or(&key).to_owned();
        folders.push(SourceFolder { count, bytes: size, label: true, ..folder(key, label, Role::Custom, 0) });
    }
    Ok(Scan { format: Format::Takeout, name, bytes, estimate, folders })
}

// ---------------------------------------------------------------- cabeçalhos

/// Valor de um cabeçalho (com as continuações), sem decodificar; só no bloco de cabeçalhos.
pub fn header_value(raw: &[u8], name: &str) -> Option<String> {
    let text = String::from_utf8_lossy(&raw[..raw.len().min(256 * 1024)]);
    let mut found: Option<String> = None;
    for line in text.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            break;
        }
        if line.starts_with([' ', '\t']) {
            if let Some(v) = found.as_mut() {
                v.push(' ');
                v.push_str(line.trim());
            }
            continue;
        }
        if found.is_some() {
            break;
        }
        if let Some((n, v)) = line.split_once(':') {
            if n.trim().eq_ignore_ascii_case(name) {
                found = Some(v.trim().to_owned());
            }
        }
    }
    found
}

/// Decodifica encoded-words (RFC 2047) de um valor solto, pelo mesmo parser da leitura.
fn decode_words(value: &str) -> String {
    if !value.contains("=?") {
        return value.to_owned();
    }
    let raw = format!("Subject: {value}\r\n\r\n");
    MessageParser::default().parse_headers(raw.as_bytes()).and_then(|m| m.subject().map(str::to_owned)).unwrap_or_else(|| value.to_owned())
}

/// Flags gravadas por Thunderbird e outros no próprio mbox. `None` para mensagem apagada que o
/// programa ainda não tinha compactado (não entra).
fn mbox_flags(raw: &[u8]) -> Option<MessageFlags> {
    let mut flags = MessageFlags { seen: true, flagged: false };
    if let Some(v) = header_value(raw, "x-mozilla-status") {
        if let Ok(bits) = u32::from_str_radix(v.trim(), 16) {
            if bits & 0x0008 != 0 {
                return None;
            }
            flags.seen = bits & 0x0001 != 0;
            flags.flagged = bits & 0x0004 != 0;
            return Some(flags);
        }
    }
    if let Some(v) = header_value(raw, "status") {
        flags.seen = v.contains('R');
    }
    if let Some(v) = header_value(raw, "x-status") {
        if v.contains('D') {
            return None;
        }
        flags.flagged = v.contains('F');
    }
    Some(flags)
}

// ---------------------------------------------------------------- mbox

enum MboxEntry {
    Message(Vec<u8>),
    TooLarge,
}

#[derive(Default)]
struct Building {
    raw: Vec<u8>,
    too_large: bool,
}

impl Building {
    fn push(&mut self, line: &[u8]) {
        if self.too_large {
            return;
        }
        if self.raw.len() + line.len() > MAX_MESSAGE_BYTES {
            self.too_large = true;
            self.raw = Vec::new();
            return;
        }
        // mboxrd: ">From " (com um ou mais ">") perde um ">".
        let unescaped = line.iter().position(|&b| b != b'>').is_some_and(|i| i > 0 && line[i..].starts_with(b"From "));
        self.raw.extend_from_slice(if unescaped { &line[1..] } else { line });
    }

    /// `last`: a última do arquivo não tem depois dela a linha em branco do separador.
    fn finish(mut self, last: bool) -> MboxEntry {
        if self.too_large {
            return MboxEntry::TooLarge;
        }
        // A linha em branco antes do próximo "From " é do separador.
        if last {
            return MboxEntry::Message(self.raw);
        }
        if self.raw.ends_with(b"\r\n") {
            self.raw.truncate(self.raw.len() - 2);
        } else if self.raw.ends_with(b"\n") {
            self.raw.pop();
        }
        MboxEntry::Message(self.raw)
    }
}

/// Percorre um mbox em streaming. Um "From " só separa depois de uma linha em branco (ou no início),
/// o que tolera corpos com "From " sem escape. `on` devolve `false` para parar.
fn each_mbox(path: &Path, cancel: &AtomicBool, mut on: impl FnMut(MboxEntry) -> bool) -> Result<(), AppError> {
    let file = File::open(path).map_err(|_| AppError::ImportSourceMissing)?;
    let mut reader = BufReader::with_capacity(1 << 20, file);
    let mut line = Vec::with_capacity(1024);
    let mut current: Option<Building> = None;
    let mut previous_blank = true;
    let mut seen = 0u32;
    loop {
        line.clear();
        let n = reader.read_until(b'\n', &mut line).map_err(|_| AppError::ImportSourceMissing)?;
        if n == 0 {
            break;
        }
        if previous_blank && line.starts_with(b"From ") {
            if let Some(b) = current.take() {
                if !on(b.finish(false)) {
                    return Ok(());
                }
                seen += 1;
                if seen.is_multiple_of(256) && cancel.load(Ordering::Relaxed) {
                    return Ok(());
                }
            }
            current = Some(Building::default());
            previous_blank = false;
            continue;
        }
        previous_blank = line == b"\n" || line == b"\r\n";
        if let Some(b) = current.as_mut() {
            b.push(&line);
        }
    }
    if let Some(b) = current {
        on(b.finish(true));
    }
    Ok(())
}

// ---------------------------------------------------------------- arquivos soltos

/// Mensagem de um `.emlx`: os bytes contados na primeira linha, e as flags do plist que vem depois.
pub fn parse_emlx(data: &[u8]) -> Option<(Vec<u8>, MessageFlags)> {
    let len = emlx_length(data)?;
    let start = data.iter().position(|&b| b == b'\n')? + 1;
    let end = start.checked_add(len)?.min(data.len());
    let raw = data[start..end].to_vec();
    let plist = String::from_utf8_lossy(&data[end..]);
    let bits = plist
        .split_once("<key>flags</key>")
        .and_then(|(_, rest)| rest.split_once("<integer>"))
        .and_then(|(_, rest)| rest.split_once("</integer>"))
        .and_then(|(n, _)| n.trim().parse::<u64>().ok())
        .unwrap_or(1);
    Some((raw, MessageFlags { seen: bits & 1 != 0, flagged: bits & 0x10 != 0 }))
}

/// Flags do nome de um arquivo Maildir (`…:2,FS`). `None` para os marcados para apagar (T).
pub fn maildir_flags(name: &str, in_new: bool) -> Option<MessageFlags> {
    let info = name.rsplit_once(":2,").or_else(|| name.rsplit_once("!2,")).map(|(_, f)| f).unwrap_or("");
    if info.contains('T') {
        return None;
    }
    Some(MessageFlags { seen: !in_new && info.contains('S'), flagged: info.contains('F') })
}

fn read_file(path: &Path) -> Result<Vec<u8>, Failure> {
    let len = std::fs::metadata(path).map(|m| m.len()).map_err(|_| Failure::Unreadable)?;
    if len as usize > MAX_MESSAGE_BYTES + 64 * 1024 {
        return Err(Failure::TooLarge);
    }
    std::fs::read(path).map_err(|_| Failure::Unreadable)
}

// ---------------------------------------------------------------- ler (passo 3)

/// Entrega as mensagens das pastas escolhidas. `on` devolve `false` para parar; `cancel` também para.
pub fn read(path: &Path, format: Format, selected: &HashSet<String>, cancel: &AtomicBool, mut on: impl FnMut(Item) -> bool) -> Result<(), AppError> {
    if format == Format::Takeout {
        return each_mbox(path, cancel, |entry| match entry {
            MboxEntry::TooLarge => on(Item::Failed(Failure::TooLarge)),
            MboxEntry::Message(raw) => {
                let (keys, flags) = takeout_targets(&labels_of(&raw));
                let folders: Vec<String> = keys.into_iter().filter(|k| selected.contains(k)).collect();
                if folders.is_empty() {
                    return true;
                }
                on(Item::Message { folders, raw, flags })
            }
        });
    }
    if format == Format::Pst {
        return crate::mapi_import::read_pst(path, selected, cancel, |r| match r {
            Ok((key, raw, flags)) => on(Item::Message { folders: vec![key], raw, flags }),
            Err(f) => on(Item::Failed(f)),
        });
    }
    let (_, located) = locate(path)?;
    for l in located.into_iter().filter(|l| selected.contains(&l.folder.key)) {
        let key = l.folder.key;
        let mut go_on = true;
        match l.place {
            Place::Mbox(p) => {
                each_mbox(&p, cancel, |entry| {
                    go_on = match entry {
                        MboxEntry::TooLarge => on(Item::Failed(Failure::TooLarge)),
                        MboxEntry::Message(raw) => match mbox_flags(&raw) {
                            Some(flags) => on(Item::Message { folders: vec![key.clone()], raw, flags }),
                            None => true,
                        },
                    };
                    go_on
                })?;
            }
            Place::Maildir(dir) => {
                'subs: for sub in ["cur", "new"] {
                    let mut files: Vec<PathBuf> = std::fs::read_dir(dir.join(sub)).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
                    files.sort();
                    for file in files {
                        if cancel.load(Ordering::Relaxed) {
                            return Ok(());
                        }
                        let Some(flags) = maildir_flags(&file_name(&file), sub == "new") else { continue };
                        go_on = match read_file(&file) {
                            Ok(raw) => on(Item::Message { folders: vec![key.clone()], raw, flags }),
                            Err(f) => on(Item::Failed(f)),
                        };
                        if !go_on {
                            break 'subs;
                        }
                    }
                }
            }
            Place::Files(files) => {
                for file in files {
                    if cancel.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    let lower = file_name(&file).to_lowercase();
                    let item = match read_file(&file) {
                        Err(f) => Item::Failed(f),
                        Ok(_) if lower.ends_with(".msg") || format == Format::Msg => match crate::mapi_import::read_msg(&file) {
                            Ok((raw, flags)) => Item::Message { folders: vec![key.clone()], raw, flags },
                            Err(f) => Item::Failed(f),
                        },
                        Ok(data) if lower.ends_with(".emlx") => match parse_emlx(&data) {
                            Some((raw, flags)) => Item::Message { folders: vec![key.clone()], raw, flags },
                            None => Item::Failed(Failure::Unreadable),
                        },
                        Ok(raw) => Item::Message { folders: vec![key.clone()], raw, flags: MessageFlags { seen: true, flagged: false } },
                    };
                    go_on = on(item);
                    if !go_on {
                        break;
                    }
                }
            }
        }
        if !go_on || cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, data: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, data).unwrap();
    }

    fn collect(path: &Path, format: Format, selected: &[&str]) -> Vec<Item> {
        let selected: HashSet<String> = selected.iter().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        read(path, format, &selected, &AtomicBool::new(false), |i| {
            out.push(i);
            true
        })
        .unwrap();
        out
    }

    const MBOX: &[u8] = b"From a@x Mon Jan  1 00:00:00 2024\nFrom: Ana <ana@x.com>\nSubject: Um\nStatus: RO\n\nCorpo um\n>From escapado\n\nFrom b@x Mon Jan  1 00:00:00 2024\nFrom: Bia <bia@x.com>\nSubject: Dois\nX-Status: F\n\nCorpo dois\nFrom no meio da linha sem branco antes\n";

    #[test]
    fn mbox_splits_unescapes_and_reads_flags() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Caixa.mbox");
        write(&path, MBOX);
        let scan = scan(&path, &AtomicBool::new(false)).unwrap();
        assert_eq!((scan.format, scan.estimate), (Format::Mbox, 2));
        assert_eq!(scan.folders[0].name, "Caixa");
        let items = collect(&path, Format::Mbox, &["Caixa"]);
        let Item::Message { raw, flags, .. } = &items[0] else { panic!() };
        let text = String::from_utf8_lossy(raw);
        assert!(text.contains("\nFrom escapado") && text.ends_with("Corpo um\nFrom escapado\n"), "{text:?}");
        assert_eq!(*flags, MessageFlags { seen: true, flagged: false });
        let Item::Message { raw, flags, .. } = &items[1] else { panic!() };
        assert!(String::from_utf8_lossy(raw).contains("From no meio"), "sem linha em branco antes, não separa");
        assert_eq!(*flags, MessageFlags { seen: true, flagged: true }, "sem Status, lida: Apple Mail e Takeout não gravam");
    }

    #[test]
    fn takeout_labels_become_folders_and_flags() {
        assert_eq!(split_labels("Inbox,\"Projetos, 2024\",Importante"), ["Inbox", "Projetos, 2024", "Importante"]);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Todos.mbox");
        let data = b"From 1@xxx Mon Jan 1 2024\nX-GM-THRID: 1\nX-Gmail-Labels: Inbox,Unread,Starred,Trabalho,Category Promotions\nFrom: a@x.com\nSubject: s\n\nOi\n\nFrom 2@xxx Mon Jan 1 2024\nX-Gmail-Labels: Spam\nFrom: b@x.com\nSubject: s\n\nPromo\n\nFrom 3@xxx Mon Jan 1 2024\nX-Gmail-Labels: =?UTF-8?Q?Viagens_=C3=A0_praia?=,Opened\nFrom: c@x.com\nSubject: s\n\nArquivada\n";
        write(&path, data);
        let scan = scan(&path, &AtomicBool::new(false)).unwrap();
        assert_eq!(scan.format, Format::Takeout);
        assert_eq!(scan.estimate, 3);
        let keys: Vec<(&str, u32, bool)> = scan.folders.iter().map(|f| (f.key.as_str(), f.count, f.selected)).collect();
        assert_eq!(keys, [("inbox", 1, true), ("all", 2, true), ("spam", 1, false), ("trash", 0, false), ("label:Trabalho", 1, true), ("label:Viagens à praia", 1, true)]);
        let items = collect(&path, Format::Takeout, &["inbox", "all", "label:Trabalho", "label:Viagens à praia"]);
        assert_eq!(items.len(), 2, "o spam não foi escolhido");
        let Item::Message { folders, flags, .. } = &items[0] else { panic!() };
        assert_eq!(folders, &["inbox", "label:Trabalho", "all"]);
        assert_eq!(*flags, MessageFlags { seen: false, flagged: true });
        let Item::Message { folders, flags, .. } = &items[1] else { panic!() };
        assert_eq!(folders, &["label:Viagens à praia", "all"]);
        assert!(flags.seen);
    }

    #[test]
    fn thunderbird_profile_has_nested_folders_and_skips_expunged() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Local Folders");
        write(&root.join("Inbox"), b"From - Mon\nX-Mozilla-Status: 0001\nFrom: a@x.com\nSubject: lida\n\nx\n\nFrom - Mon\nX-Mozilla-Status: 0009\nFrom: a@x.com\nSubject: apagada\n\ny\n");
        write(&root.join("Inbox.msf"), b"// <!-- <mdb:mork");
        write(&root.join("Trash"), b"From - Mon\nFrom: a@x.com\nSubject: t\n\nz\n");
        write(&root.join("Trabalho.sbd").join("Clientes"), b"From - Mon\nX-Mozilla-Status: 0004\nFrom: a@x.com\nSubject: c\n\nw\n");
        write(&root.join("notas.txt"), b"nada");
        let scan = scan(&root, &AtomicBool::new(false)).unwrap();
        assert_eq!(scan.format, Format::Thunderbird);
        let folders: Vec<(&str, Role, u32, bool)> = scan.folders.iter().map(|f| (f.key.as_str(), f.role, f.depth, f.selected)).collect();
        assert_eq!(folders, [("Inbox", Role::Inbox, 0, true), ("Trabalho/Clientes", Role::Custom, 1, true), ("Trash", Role::Trash, 0, false)]);
        let items = collect(&root, Format::Thunderbird, &["Inbox", "Trabalho/Clientes"]);
        assert_eq!(items.len(), 2, "a apagada e não compactada fica de fora");
        let Item::Message { flags, folders, .. } = &items[1] else { panic!() };
        assert_eq!((folders[0].as_str(), *flags), ("Trabalho/Clientes", MessageFlags { seen: false, flagged: true }));
    }

    #[test]
    fn maildir_and_apple_mail_folders() {
        let dir = tempfile::tempdir().unwrap();
        let md = dir.path().join("Maildir");
        write(&md.join("cur").join("1.host!2,FS"), b"From: a@x.com\nSubject: um\n\nx");
        write(&md.join("new").join("2.host"), b"From: a@x.com\nSubject: dois\n\nx");
        write(&md.join("cur").join("3.host!2,ST"), b"From: a@x.com\nSubject: apagar\n\nx");
        std::fs::create_dir_all(md.join("tmp")).unwrap();
        write(&md.join(".Sent").join("cur").join("4.host!2,S"), b"From: a@x.com\nSubject: env\n\nx");
        std::fs::create_dir_all(md.join(".Sent").join("new")).unwrap();
        let scan = scan(&md, &AtomicBool::new(false)).unwrap();
        assert_eq!(scan.format, Format::Maildir);
        assert_eq!(scan.folders.iter().map(|f| (f.key.as_str(), f.role)).collect::<Vec<_>>(), [("Maildir", Role::Inbox), ("Maildir/Sent", Role::Sent)]);
        let items = collect(&md, Format::Maildir, &["Maildir"]);
        let flags: Vec<MessageFlags> = items.iter().map(|i| match i { Item::Message { flags, .. } => *flags, _ => panic!() }).collect();
        assert_eq!(flags, [MessageFlags { seen: true, flagged: true }, MessageFlags { seen: false, flagged: false }]);

        let apple = dir.path().join("Apple");
        write(&apple.join("Entrada.mbox").join("mbox"), MBOX);
        let message = b"From: a@x.com\nSubject: emlx\n\ncorpo";
        let mut emlx = format!("{}\n", message.len()).into_bytes();
        emlx.extend_from_slice(message);
        emlx.extend_from_slice(b"<?xml version=\"1.0\"?><plist><dict><key>flags</key><integer>17</integer></dict></plist>");
        write(&apple.join("Arquivo.mbox").join("ABC").join("Data").join("Messages").join("1.emlx"), &emlx);
        let scan = super::scan(&apple, &AtomicBool::new(false)).unwrap();
        assert_eq!(scan.format, Format::AppleMail);
        assert_eq!(scan.folders.iter().map(|f| (f.key.as_str(), f.count)).collect::<Vec<_>>(), [("Arquivo", 1), ("Entrada", 2)]);
        let items = collect(&apple, Format::AppleMail, &["Arquivo"]);
        let Item::Message { raw, flags, .. } = &items[0] else { panic!() };
        assert_eq!(raw.as_slice(), message);
        assert_eq!(*flags, MessageFlags { seen: true, flagged: true });
    }

    #[test]
    fn single_files_are_recognized_by_content() {
        let dir = tempfile::tempdir().unwrap();
        let eml = dir.path().join("sem extensao");
        write(&eml, b"Received: from x\r\nFrom: a@x.com\r\nSubject: Oi\r\n\r\nCorpo");
        assert_eq!(scan(&eml, &AtomicBool::new(false)).unwrap().format, Format::Eml);
        let txt = dir.path().join("nota.eml");
        write(&txt, b"Lista de compras:\nleite\n");
        assert_eq!(scan(&txt, &AtomicBool::new(false)), Err(AppError::ImportUnrecognized));
        let zip = dir.path().join("takeout.zip");
        write(&zip, b"PK\x03\x04resto");
        assert_eq!(scan(&zip, &AtomicBool::new(false)), Err(AppError::ImportUnrecognized));
        let pst = dir.path().join("a.pst");
        write(&pst, b"!BDNxxxx");
        assert_eq!(detect_file(&pst), Ok(Format::Pst));
        assert_eq!(scan(&dir.path().join("sumiu"), &AtomicBool::new(false)), Err(AppError::ImportSourceMissing));
    }

    #[test]
    fn oversized_messages_fail_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grande.mbox");
        let mut data = b"From a\nFrom: a@x.com\nSubject: grande\n\n".to_vec();
        let line = vec![b'x'; 1023];
        for _ in 0..(MAX_MESSAGE_BYTES / 1024 + 2) {
            data.extend_from_slice(&line);
            data.push(b'\n');
        }
        data.extend_from_slice(b"\nFrom b\nFrom: b@x.com\nSubject: pequena\n\nok\n");
        write(&path, &data);
        let items = collect(&path, Format::Mbox, &["grande"]);
        assert!(matches!(items[0], Item::Failed(Failure::TooLarge)));
        assert!(matches!(&items[1], Item::Message { raw, .. } if raw.ends_with(b"ok\n")));
    }
}
