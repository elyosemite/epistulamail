//! Prévia da lista na densidade Confortável (US7, CA7, D2): o trecho do início do texto e se há
//! arquivos ou imagens anexados. Vem do `BODYSTRUCTURE` e dos primeiros 2 KB da parte de texto,
//! sem baixar a mensagem inteira; com o corpo já salvo, sai dele. Nada daqui vai para log.

use std::borrow::Cow;

use async_imap::imap_proto::{BodyStructure, ContentEncoding};
use mail_parser::{MessageParser, MimeHeaders, PartType};

/// Bytes da parte de texto buscados para montar o trecho.
pub const PEEK_BYTES: u32 = 2048;
pub const SNIPPET_CHARS: usize = 200;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Preview {
    pub snippet: String,
    pub has_files: bool,
    pub has_images: bool,
}

/// Parte de texto escolhida para o trecho: a seção IMAP (`1`, `1.2`…), como decodificar e se é HTML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPart {
    pub section: String,
    pub encoding: String,
    pub charset: String,
    pub html: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Structure {
    pub has_files: bool,
    pub has_images: bool,
    pub text: Option<TextPart>,
}

fn param<'a>(params: &'a Option<Vec<(Cow<'_, str>, Cow<'_, str>)>>, name: &str) -> Option<&'a str> {
    params.as_ref()?.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_ref())
}

fn encoding_name(e: &ContentEncoding<'_>) -> String {
    match e {
        ContentEncoding::Base64 => "base64".into(),
        ContentEncoding::QuotedPrintable => "quoted-printable".into(),
        ContentEncoding::EightBit => "8bit".into(),
        ContentEncoding::Binary => "binary".into(),
        _ => "7bit".into(),
    }
}

/// Só letras, números e `-_.:` (o charset vai para um cabeçalho montado aqui).
fn clean_charset(raw: Option<&str>) -> String {
    raw.map(|c| c.chars().filter(|ch| ch.is_ascii_alphanumeric() || "-_.:".contains(*ch)).take(40).collect::<String>())
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| "utf-8".into())
}

/// Percorre a estrutura: a primeira parte `text/plain` que não é anexo (senão a primeira
/// `text/html`) dá o trecho; imagens fora de `multipart/related` (as embutidas no HTML) e partes
/// com nome de arquivo ou `attachment` contam como anexos.
pub fn analyze(bs: &BodyStructure<'_>) -> Structure {
    let mut out = Structure::default();
    let mut html: Option<TextPart> = None;
    walk(bs, &mut Vec::new(), None, true, &mut out, &mut html);
    if out.text.is_none() {
        out.text = html;
    }
    out
}

fn walk(bs: &BodyStructure<'_>, path: &mut Vec<u32>, parent: Option<&str>, root: bool, out: &mut Structure, html: &mut Option<TextPart>) {
    let section = |path: &Vec<u32>| if path.is_empty() { "1".to_owned() } else { path.iter().map(u32::to_string).collect::<Vec<_>>().join(".") };
    match bs {
        BodyStructure::Multipart { common, bodies, .. } => {
            let subtype = common.ty.subtype.to_ascii_lowercase();
            for (i, body) in bodies.iter().enumerate() {
                path.push(i as u32 + 1);
                walk(body, path, Some(&subtype), false, out, html);
                path.pop();
            }
        }
        BodyStructure::Text { common, other, .. } | BodyStructure::Basic { common, other, .. } => {
            let ty = common.ty.ty.to_ascii_lowercase();
            let subtype = common.ty.subtype.to_ascii_lowercase();
            let disposition = common.disposition.as_ref().map(|d| d.ty.to_ascii_lowercase());
            let named = param(&common.ty.params, "name").is_some()
                || common.disposition.as_ref().is_some_and(|d| param(&d.params, "filename").is_some());
            let attachment = disposition.as_deref() == Some("attachment") || (named && disposition.as_deref() != Some("inline"));
            if ty == "text" && !attachment && (subtype == "plain" || subtype == "html") {
                let part = TextPart {
                    section: section(path),
                    encoding: encoding_name(&other.transfer_encoding),
                    charset: clean_charset(param(&common.ty.params, "charset")),
                    html: subtype == "html",
                };
                if subtype == "plain" && out.text.is_none() {
                    out.text = Some(part);
                } else if subtype == "html" && html.is_none() {
                    *html = Some(part);
                }
            } else if ty == "image" {
                if attachment || parent != Some("related") {
                    out.has_images = true;
                }
            } else if !root && (attachment || named) {
                out.has_files = true;
            }
        }
        BodyStructure::Message { .. } => {
            if !root {
                out.has_files = true;
            }
        }
    }
}

/// Corta o fim incompleto de um pedaço codificado (os 2 KB podem parar no meio de um caractere).
fn trim_partial(bytes: &[u8], encoding: &str) -> Vec<u8> {
    match encoding {
        "base64" => {
            let clean: Vec<u8> = bytes.iter().copied().filter(|b| !b.is_ascii_whitespace()).collect();
            let keep = clean.len() - clean.len() % 4;
            clean[..keep].to_vec()
        }
        "quoted-printable" => {
            let mut v = bytes.to_vec();
            if let Some(i) = v.iter().rposition(|b| *b == b'=') {
                if v.len() - i < 3 {
                    v.truncate(i);
                }
            }
            v
        }
        _ => bytes.to_vec(),
    }
}

/// Texto puro do trecho: sem linhas citadas (`>`), espaços normalizados, até `SNIPPET_CHARS`.
pub fn clean_snippet(text: &str) -> String {
    let joined = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('>'))
        .take_while(|l| *l != "--")
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = String::new();
    let mut space = false;
    for ch in joined.chars() {
        if ch.is_whitespace() || ch.is_control() || ch == '\u{200c}' || ch == '\u{a0}' {
            space = !out.is_empty();
            continue;
        }
        if space {
            out.push(' ');
            space = false;
        }
        out.push(ch);
        if out.chars().count() >= SNIPPET_CHARS {
            break;
        }
    }
    out
}

/// Decodifica o começo da parte de texto montando uma parte MIME só com os cabeçalhos conhecidos.
pub fn snippet_from_part(bytes: &[u8], part: &TextPart) -> String {
    let mut raw = format!(
        "Content-Type: text/{}; charset=\"{}\"\r\nContent-Transfer-Encoding: {}\r\n\r\n",
        if part.html { "html" } else { "plain" },
        part.charset,
        part.encoding
    )
    .into_bytes();
    raw.extend(trim_partial(bytes, &part.encoding));
    MessageParser::default().parse(&raw).and_then(|m| m.body_text(0).map(|t| clean_snippet(&t))).unwrap_or_default()
}

/// Prévia a partir da mensagem completa (corpo já salvo neste computador).
pub fn from_raw(raw: &[u8]) -> Preview {
    let Some(message) = MessageParser::default().parse(raw) else { return Preview::default() };
    let snippet = message.body_text(0).map(|t| clean_snippet(&t)).unwrap_or_default();
    let mut preview = Preview { snippet, ..Default::default() };
    for part in message.attachments() {
        let image = part.content_type().is_some_and(|ct| ct.ctype().eq_ignore_ascii_case("image"));
        let inline = part.content_disposition().is_some_and(|d| d.is_inline()) && part.content_id().is_some();
        match (&part.body, image) {
            (PartType::Message(_), _) => preview.has_files = true,
            (_, true) if !inline => preview.has_images = true,
            (_, true) => {}
            _ => preview.has_files = true,
        }
    }
    preview
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_skips_quotes_signature_and_extra_spaces() {
        let text = "Oi, João.\r\n\r\nSegue   a versão 3.\r\n> citado\r\n--\r\nMarina";
        assert_eq!(clean_snippet(text), "Oi, João. Segue a versão 3.");
        assert_eq!(clean_snippet(&"a".repeat(500)).chars().count(), SNIPPET_CHARS);
        assert_eq!(clean_snippet(""), "");
    }

    #[test]
    fn partial_base64_and_quoted_printable_decode() {
        let part = TextPart { section: "1".into(), encoding: "base64".into(), charset: "utf-8".into(), html: false };
        // "Olá, mundo! Tudo bem?" em base64, cortado no meio de um bloco.
        let b64 = b"T2zDoSwgbXVuZG8hIFR1ZG8gYmVtPw==";
        assert_eq!(snippet_from_part(&b64[..20], &part), "Olá, mundo! Tu", "20 caracteres = 15 bytes");
        assert_eq!(snippet_from_part(&b64[..22], &part), "Olá, mundo! Tu", "bloco incompleto é descartado");
        let qp = TextPart { encoding: "quoted-printable".into(), charset: "iso-8859-1".into(), ..part.clone() };
        assert_eq!(snippet_from_part(b"Ol=E1, mundo=", &qp), "Olá, mundo");
        let html = TextPart { html: true, encoding: "7bit".into(), ..part };
        assert_eq!(snippet_from_part(b"<p>Oi <b>Ana</b></p><p>Tudo bem?</p>", &html), "Oi Ana Tudo bem?");
    }

    fn structure(fetch: &[u8]) -> Structure {
        use async_imap::imap_proto::{AttributeValue, Response};
        let (_, response) = async_imap::imap_proto::parser::parse_response(fetch).unwrap();
        let Response::Fetch(_, attrs) = response else { panic!("não é FETCH") };
        let bs = attrs.iter().find_map(|a| if let AttributeValue::BodyStructure(bs) = a { Some(bs) } else { None }).unwrap();
        analyze(bs)
    }

    #[test]
    fn structure_picks_the_text_part_and_attachments() {
        // multipart/mixed: (alternative: plain + html), pdf anexado
        let s = structure(b"* 1 FETCH (UID 7 BODYSTRUCTURE (((\"text\" \"plain\" (\"charset\" \"iso-8859-1\") NIL NIL \"quoted-printable\" 120 4 NIL NIL NIL)(\"text\" \"html\" (\"charset\" \"utf-8\") NIL NIL \"base64\" 900 12 NIL NIL NIL) \"alternative\" (\"boundary\" \"a\") NIL NIL)(\"application\" \"pdf\" (\"name\" \"c.pdf\") NIL NIL \"base64\" 5000 NIL (\"attachment\" (\"filename\" \"c.pdf\")) NIL) \"mixed\" (\"boundary\" \"b\") NIL NIL))\r\n");
        assert_eq!(s.text, Some(TextPart { section: "1.1".into(), encoding: "quoted-printable".into(), charset: "iso-8859-1".into(), html: false }));
        assert!(s.has_files && !s.has_images);

        // Só HTML, com logo embutido (related) e uma foto anexada.
        let s = structure(b"* 2 FETCH (UID 8 BODYSTRUCTURE (((\"text\" \"html\" NIL NIL NIL \"7bit\" 300 6 NIL NIL NIL)(\"image\" \"png\" NIL \"<logo>\" NIL \"base64\" 800 NIL (\"inline\" NIL) NIL) \"related\" (\"boundary\" \"r\") NIL NIL)(\"image\" \"jpeg\" (\"name\" \"f.jpg\") NIL NIL \"base64\" 9000 NIL (\"attachment\" (\"filename\" \"f.jpg\")) NIL) \"mixed\" (\"boundary\" \"m\") NIL NIL))\r\n");
        assert_eq!(s.text.as_ref().map(|t| (t.section.as_str(), t.html)), Some(("1.1", true)));
        assert!(s.has_images && !s.has_files);

        // Mensagem de uma parte só: a seção é 1.
        let s = structure(b"* 3 FETCH (UID 9 BODYSTRUCTURE (\"text\" \"plain\" (\"charset\" \"utf-8\") NIL NIL \"7bit\" 20 1 NIL NIL NIL))\r\n");
        assert_eq!(s.text.map(|t| t.section), Some("1".into()));
        assert!(!s.has_files && !s.has_images);
    }

    #[test]
    fn charset_is_sanitized() {
        assert_eq!(clean_charset(Some("utf-8\"\r\nX-Evil: 1")), "utf-8X-Evil:1");
        assert_eq!(clean_charset(None), "utf-8");
    }

    #[test]
    fn preview_from_full_message() {
        let raw = b"From: a@x.com\r\nSubject: s\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=b\r\n\r\n\
--b\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nOi, segue o contrato.\r\n\
--b\r\nContent-Type: application/pdf; name=c.pdf\r\nContent-Disposition: attachment; filename=c.pdf\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERg==\r\n\
--b\r\nContent-Type: image/jpeg; name=f.jpg\r\nContent-Disposition: attachment; filename=f.jpg\r\nContent-Transfer-Encoding: base64\r\n\r\n/9j/\r\n\
--b--\r\n";
        let p = from_raw(raw);
        assert_eq!(p.snippet, "Oi, segue o contrato.");
        assert!(p.has_files && p.has_images);
        let plain = from_raw(b"From: a@x.com\r\nSubject: s\r\n\r\nSo texto.");
        assert_eq!(plain, Preview { snippet: "So texto.".into(), has_files: false, has_images: false });
    }
}
