//! Links do corpo da mensagem (US10, CA8). Antes de abrir no navegador: os parâmetros de rastreamento
//! conhecidos saem do endereço (D8), e um link cujo texto parece um endereço de outro domínio pede
//! confirmação, com os dois endereços à vista. Logs: nunca o endereço, só quantos parâmetros saíram.

use reqwest::Url;
use serde::Serialize;

/// Parâmetros de rastreamento retirados sem perguntar (lista fixa e curta, para não quebrar links de
/// confirmação que dependem de outros parâmetros). `utm_` vale como prefixo.
const TRACKING: &[&str] = &[
    "fbclid", "gclid", "dclid", "gbraid", "wbraid", "msclkid", "yclid", "twclid", "igshid", "mc_cid", "mc_eid", "_hsenc", "_hsmi", "mkt_tok",
];

fn is_tracking(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.starts_with("utm_") || TRACKING.contains(&name.as_str())
}

/// Tira os parâmetros de rastreamento de um link `http`/`https`. Devolve quantos saíram.
pub fn strip_tracking(url: &mut Url) -> usize {
    if !matches!(url.scheme(), "http" | "https") || url.query().is_none() {
        return 0;
    }
    let pairs: Vec<(String, String)> = url.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
    let kept: Vec<&(String, String)> = pairs.iter().filter(|(k, _)| !is_tracking(k)).collect();
    let removed = pairs.len() - kept.len();
    if removed == 0 {
        return 0;
    }
    if kept.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(kept.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    }
    removed
}

/// O domínio que o texto do link mostra, se o texto parece um endereço ("www.banco.com.br",
/// "https://banco.com/entrar"). Texto comum ("Clique aqui") devolve `None`.
pub fn shown_domain(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() || text.contains(char::is_whitespace) {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    let rest = lower.strip_prefix("https://").or_else(|| lower.strip_prefix("http://")).unwrap_or(&lower);
    let host = rest.split(['/', '?', '#']).next()?.split(':').next()?.trim_end_matches('.');
    let labels: Vec<&str> = host.split('.').collect();
    let looks_like_domain = labels.len() >= 2
        && labels.iter().all(|l| !l.is_empty() && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        && labels.last().is_some_and(|tld| tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()));
    looks_like_domain.then(|| host.to_owned())
}

fn same_site(shown: &str, target: &str) -> bool {
    let shown = shown.strip_prefix("www.").unwrap_or(shown);
    let target = target.trim_end_matches('.').to_ascii_lowercase();
    target == shown || target.ends_with(&format!(".{shown}"))
}

/// Link cujo texto mostra um domínio e leva a outro (CA8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Misleading {
    /// O que o texto mostra.
    pub shown: String,
    /// O endereço que vai abrir, já sem rastreamento.
    pub target: String,
    /// O domínio de destino, para destacar.
    pub target_host: String,
}

pub fn misleading(text: &str, url: &Url) -> Option<Misleading> {
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let shown = shown_domain(text)?;
    let host = url.host_str()?;
    (!same_site(&shown, host)).then(|| Misleading { shown: text.trim().to_owned(), target: url.to_string(), target_host: host.to_owned() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean(s: &str) -> (String, usize) {
        let mut url = Url::parse(s).unwrap();
        let n = strip_tracking(&mut url);
        (url.to_string(), n)
    }

    #[test]
    fn tracking_parameters_are_removed_and_the_rest_kept() {
        assert_eq!(clean("https://loja.com/p?id=7&utm_source=news&utm_medium=email&fbclid=x"), ("https://loja.com/p?id=7".into(), 3));
        assert_eq!(clean("https://loja.com/p?UTM_Campaign=a&gclid=b"), ("https://loja.com/p".into(), 2));
        assert_eq!(clean("https://loja.com/p?id=7#topo"), ("https://loja.com/p?id=7#topo".into(), 0), "sem rastreamento, nada muda");
        // Links de confirmação continuam funcionando.
        assert_eq!(clean("https://conta.com/confirmar?token=abc&email=a%40b.com"), ("https://conta.com/confirmar?token=abc&email=a%40b.com".into(), 0));
        assert_eq!(clean("mailto:a@b.com?subject=utm_x"), ("mailto:a@b.com?subject=utm_x".into(), 0));
    }

    #[test]
    fn text_that_looks_like_an_address() {
        assert_eq!(shown_domain("www.meubanco.com.br").as_deref(), Some("www.meubanco.com.br"));
        assert_eq!(shown_domain("https://Banco.com/entrar").as_deref(), Some("banco.com"));
        assert_eq!(shown_domain("banco.com:443/x").as_deref(), Some("banco.com"));
        for not in ["Clique aqui", "", "Acesse banco.com agora", "v1.2", "arquivo.pdf1", "a..b"] {
            assert_eq!(shown_domain(not), None, "{not}");
        }
    }

    #[test]
    fn link_to_another_domain_is_flagged() {
        let url = Url::parse("https://meubanco-seguro.xyz/entrar").unwrap();
        let m = misleading("www.meubanco.com.br", &url).unwrap();
        assert_eq!((m.shown.as_str(), m.target_host.as_str()), ("www.meubanco.com.br", "meubanco-seguro.xyz"));
        let tricky = Url::parse("https://meubanco.com.br.golpe.xyz/").unwrap();
        assert!(misleading("meubanco.com.br", &tricky).is_some(), "o domínio mostrado como subdomínio do golpe");
    }

    #[test]
    fn same_site_and_plain_text_open_directly() {
        let url = Url::parse("https://login.meubanco.com.br/entrar").unwrap();
        assert_eq!(misleading("www.meubanco.com.br", &url), None);
        assert_eq!(misleading("meubanco.com.br", &Url::parse("https://www.meubanco.com.br").unwrap()), None);
        assert_eq!(misleading("Clique aqui", &Url::parse("https://qualquer.xyz").unwrap()), None);
        assert_eq!(misleading("ana@lumen.com", &Url::parse("mailto:outra@x.com").unwrap()), None);
    }
}
