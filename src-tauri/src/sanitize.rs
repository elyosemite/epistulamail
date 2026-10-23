//! Sanitização do HTML de e-mails de terceiros (US2, CA4).
//!
//! Invariantes de segurança:
//! - Allowlist de tags e atributos (`ammonia`); tudo fora dela é removido, inclusive o conteúdo
//!   de `<script>` e `<style>`.
//! - Nenhum recurso remoto sobrevive sem consentimento: `src` http(s) de `<img>` só é mantido
//!   com `allow_remote_images`, e CSS com `url(`/`@import`/`expression(` é descartado.
//! - Links só com `http`, `https` ou `mailto`; URLs relativas são removidas.
//! - O documento final leva uma CSP própria e é exibido num iframe sem `allow-scripts`.

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use ammonia::{Builder, UrlRelative};

const TAGS: &[&str] = &[
    "a", "abbr", "b", "big", "blockquote", "br", "caption", "center", "cite", "code", "col", "colgroup", "dd", "del",
    "div", "dl", "dt", "em", "font", "h1", "h2", "h3", "h4", "h5", "h6", "hr", "i", "img", "ins", "kbd", "li", "ol",
    "p", "pre", "q", "s", "small", "span", "strike", "strong", "sub", "sup", "table", "tbody", "td", "tfoot", "th",
    "thead", "tr", "tt", "u", "ul",
];

const GENERIC_ATTRIBUTES: &[&str] = &["style", "align", "dir", "lang", "title"];

const TAG_ATTRIBUTES: &[(&str, &[&str])] = &[
    ("a", &["href", "name"]),
    ("img", &["src", "alt", "width", "height"]),
    ("table", &["width", "border", "cellpadding", "cellspacing", "bgcolor", "summary"]),
    ("td", &["width", "height", "colspan", "rowspan", "valign", "bgcolor", "nowrap"]),
    ("th", &["width", "height", "colspan", "rowspan", "valign", "bgcolor", "nowrap"]),
    ("tr", &["valign", "bgcolor"]),
    ("col", &["span", "width"]),
    ("colgroup", &["span", "width"]),
    ("font", &["color", "face", "size"]),
    ("ol", &["start", "type"]),
    ("li", &["value"]),
];

/// Formatos de imagem inline aceitos em `data:`. SVG fica de fora por poder carregar recursos.
const INLINE_IMAGE_PREFIXES: &[&str] = &["data:image/png", "data:image/jpeg", "data:image/jpg", "data:image/gif", "data:image/webp"];

/// Trechos que fazem uma declaração CSS ser descartada: carregam recursos ou executam código
/// em motores antigos. A barra invertida entra porque escapes CSS escondem `url(`.
const FORBIDDEN_CSS: &[&str] = &["url(", "image-set(", "expression(", "@import", "behavior", "-moz-binding", "javascript:", "\\"];

pub struct SanitizedHtml {
    pub html: String,
    pub blocked_remote_images: usize,
}

fn is_remote(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("//")
}

fn filter_style(value: &str) -> Option<String> {
    let kept: Vec<&str> = value
        .split(';')
        .map(str::trim)
        .filter(|decl| !decl.is_empty())
        .filter(|decl| {
            let lower = decl.to_ascii_lowercase();
            !FORBIDDEN_CSS.iter().any(|bad| lower.contains(bad))
        })
        .collect();
    (!kept.is_empty()).then(|| kept.join("; "))
}

pub fn sanitize_email_html(input: &str, allow_remote_images: bool) -> SanitizedHtml {
    let blocked = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&blocked);

    let mut builder = Builder::default();
    builder
        .tags(TAGS.iter().copied().collect::<HashSet<_>>())
        .generic_attributes(GENERIC_ATTRIBUTES.iter().copied().collect::<HashSet<_>>())
        .tag_attributes(
            TAG_ATTRIBUTES
                .iter()
                .map(|(tag, attrs)| (*tag, attrs.iter().copied().collect::<HashSet<_>>()))
                .collect(),
        )
        .url_schemes(["http", "https", "mailto", "data"].into_iter().collect())
        .url_relative(UrlRelative::Deny)
        .link_rel(Some("noopener noreferrer"))
        .strip_comments(true)
        .attribute_filter(move |element, attribute, value| match (element, attribute) {
            ("img", "src") => {
                if is_remote(value) {
                    if allow_remote_images {
                        Some(Cow::Borrowed(value))
                    } else {
                        counter.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                } else {
                    let lower = value.trim_start().to_ascii_lowercase();
                    INLINE_IMAGE_PREFIXES.iter().any(|p| lower.starts_with(p)).then_some(Cow::Borrowed(value))
                }
            }
            ("a", "href") => (!value.trim_start().to_ascii_lowercase().starts_with("data:")).then_some(Cow::Borrowed(value)),
            (_, "style") => filter_style(value).map(Cow::Owned),
            _ => Some(Cow::Borrowed(value)),
        });

    let html = builder.clean(input).to_string();
    SanitizedHtml { html, blocked_remote_images: blocked.load(Ordering::Relaxed) }
}

/// Documento completo para o `srcdoc` do iframe, com CSP própria. Sem cores fixas: o quadro
/// do DS (`.message__frame`) fornece o fundo de papel, e o texto usa o padrão do navegador.
pub fn email_document(clean_html: &str, allow_remote_images: bool) -> String {
    let img_src = if allow_remote_images { "data: https: http:" } else { "data:" };
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\">\
<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; img-src {img_src}; font-src data:\">\
<style>html,body{{margin:0}}body{{padding:16px;font-family:system-ui,-apple-system,'Segoe UI',Roboto,Arial,sans-serif;font-size:14px;line-height:1.5;overflow-wrap:anywhere}}img{{max-width:100%;height:auto}}table{{max-width:100%}}</style>\
</head><body>{clean_html}</body></html>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn clean(input: &str) -> String {
        sanitize_email_html(input, false).html.to_ascii_lowercase()
    }

    /// Nenhuma tag da saída pode ter atributo de evento (`on*=`).
    fn has_event_handler(html: &str) -> bool {
        html.split('<').skip(1).any(|tag| {
            let tag = tag.split('>').next().unwrap_or("");
            tag.split(|c: char| c.is_whitespace() || c == '/').any(|tok| tok.starts_with("on") && tok.contains('='))
        })
    }

    /// Texto da mensagem sai escapado (`&lt;`), então só o que está dentro de tags pode ser ativo:
    /// nomes de tag perigosos são procurados no documento todo, e esquemas/CSS só dentro das tags.
    fn assert_safe(input: &str) {
        let out = clean(input);
        for bad in ["<script", "<iframe", "<object", "<embed", "<form", "<svg", "<math", "<meta", "<base", "<link", "<style"] {
            assert!(!out.contains(bad), "entrada {input:?} produziu {bad:?}: {out}");
        }
        for tag in out.split('<').skip(1).map(|t| t.split('>').next().unwrap_or("")) {
            for bad in ["javascript:", "vbscript:", "@import", "url(", "expression(", "data:text"] {
                assert!(!tag.contains(bad), "entrada {input:?} produziu {bad:?} na tag <{tag}>");
            }
        }
        assert!(!has_event_handler(&out), "atributo de evento em {out}");
    }

    #[test]
    fn attack_corpus_is_neutralized() {
        let corpus = [
            "<script>alert(1)</script>",
            "<SCRIPT SRC=//evil/x.js></SCRIPT>",
            "<img src=x onerror=alert(1)>",
            "<a href=\"javascript:alert(1)\">x</a>",
            "<a href=\"  JaVaScRiPt:alert(1)\">x</a>",
            "<a href=\"jav&#x09;ascript:alert(1)\">x</a>",
            "<a href=\"vbscript:msgbox(1)\">x</a>",
            "<a href=\"data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==\">x</a>",
            "<svg onload=alert(1)><circle/></svg>",
            "<math><a xlink:href=\"javascript:alert(1)\">x</a></math>",
            "<iframe srcdoc=\"<script>alert(1)</script>\"></iframe>",
            "<meta http-equiv=\"refresh\" content=\"0;url=https://evil\">",
            "<base href=\"https://evil/\">",
            "<form action=\"https://evil\"><input name=p></form>",
            "<object data=\"x.swf\"></object><embed src=\"x.swf\">",
            "<link rel=stylesheet href=\"https://evil/x.css\">",
            "<style>@import url(https://evil/x.css); body{background:url(https://evil/t.gif)}</style>",
            "<div style=\"background:url(https://evil/t.gif)\">x</div>",
            "<div style=\"background:u\\72l(https://evil/t.gif)\">x</div>",
            "<div style=\"width:expression(alert(1))\">x</div>",
            "<p style=\"behavior:url(x.htc)\">x</p>",
            "<!--[if mso]><script>alert(1)</script><![endif]-->",
            "<body onload=alert(1)>x</body>",
            "<img src=\"data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=\">",
            "<<script>script>alert(1)<</script>/script>",
            "<div onmouseover=\"alert(1)\" style=\"color:red\">x</div>",
            "<table background=\"https://evil/t.gif\"><tr><td background=\"https://evil/t.gif\">x</td></tr></table>",
        ];
        for input in corpus {
            assert_safe(input);
        }
    }

    #[test]
    fn remote_images_are_blocked_and_counted() {
        let input = "<img src=\"https://tracker/pixel.gif\"><img src=\"http://a/b.png\"><img src=\"//cdn/c.png\"><img srcset=\"https://x/1.png 1x\">";
        let out = sanitize_email_html(input, false);
        // `//cdn` é relativa ao protocolo: o ammonia a remove sempre (UrlRelative::Deny), antes do
        // filtro; por isso não entra na contagem do "Mostrar imagens". `srcset` também nunca passa.
        assert_eq!(out.blocked_remote_images, 2);
        assert!(!out.html.contains("tracker"));
        assert!(!out.html.contains("cdn/c.png"));
        assert!(!out.html.contains("srcset"));
    }

    #[test]
    fn remote_images_survive_only_with_consent() {
        let out = sanitize_email_html("<img src=\"https://cdn.lumen.com/logo.png\" alt=\"Lumen\">", true);
        assert!(out.html.contains("https://cdn.lumen.com/logo.png"));
        assert_eq!(out.blocked_remote_images, 0);
    }

    #[test]
    fn inline_raster_images_are_kept() {
        let out = sanitize_email_html("<img src=\"data:image/png;base64,iVBORw0KGgo=\">", false);
        assert!(out.html.contains("data:image/png"));
    }

    #[test]
    fn safe_layout_and_links_are_preserved() {
        let out = sanitize_email_html(
            "<table width=\"600\" cellpadding=\"8\"><tr><td style=\"color:#333;padding:4px\"><a href=\"https://lumen.com\">Lumen</a> <a href=\"mailto:a@b.com\">fale</a> <a href=\"/relativo\">r</a></td></tr></table>",
            false,
        );
        assert!(out.html.contains("<table width=\"600\" cellpadding=\"8\">"));
        assert!(out.html.contains("color:#333; padding:4px"));
        assert!(out.html.contains("href=\"https://lumen.com\""));
        assert!(out.html.contains("href=\"mailto:a@b.com\""));
        assert!(out.html.contains("rel=\"noopener noreferrer\""));
        assert!(!out.html.contains("/relativo"));
    }

    #[test]
    fn document_csp_follows_consent() {
        assert!(email_document("x", false).contains("img-src data:;"));
        assert!(email_document("x", true).contains("img-src data: https: http:;"));
        assert!(email_document("x", false).contains("default-src 'none'"));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        #[test]
        fn arbitrary_text_never_yields_active_content(input in ".{0,400}") {
            assert_safe(&input);
        }

        #[test]
        fn html_like_soup_never_yields_active_content(
            parts in proptest::collection::vec(
                prop_oneof![
                    Just("<script>"), Just("</script>"), Just("<img src=x onerror=alert(1)>"), Just("<a href='javascript:x'>"),
                    Just("<svg>"), Just("<style>"), Just("url(http://e)"), Just("<div style='background:url(//e)'>"),
                    Just("<!--"), Just("-->"), Just("<"), Just(">"), Just("\""), Just("'"), Just("&#x6a;"), Just("on"),
                    Just("load="), Just("<iframe>"), Just("<p>"), Just("texto"),
                ],
                0..40,
            )
        ) {
            assert_safe(&parts.concat());
        }
    }
}
