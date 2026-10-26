//! Buscar a chave OpenPGP de alguém na internet (US13, CA3; D5: só com permissão). Primeiro no
//! domínio do destinatário (WKD, no método avançado e no direto), depois em keys.openpgp.org.
//! Quem responde fica sabendo que alguém procurou aquele e-mail: por isso a busca nunca é sozinha.
//! Logs: só a origem que respondeu, nunca o endereço.

use reqwest::Url;
use sha1::{Digest, Sha1};

use crate::error::AppError;

const MAX_KEY_BYTES: usize = 512 * 1024;

/// z-base-32 (RFC 6189), usado pelo WKD para o hash da parte local do e-mail.
fn zbase32(data: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ybndrfg8ejkmcpqxot1uwisza345h769";
    let mut out = String::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for &b in data {
        buffer = (buffer << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            out.push(ALPHABET[((buffer >> (bits - 5)) & 31) as usize] as char);
            bits -= 5;
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

/// Os endereços WKD (avançado e direto) de um e-mail.
pub fn wkd_urls(email: &str) -> Option<[Url; 2]> {
    let (local, domain) = email.trim().rsplit_once('@')?;
    let domain = domain.to_lowercase();
    if local.is_empty() || domain.is_empty() || !domain.contains('.') || domain.contains(['/', '?', '#', ':']) {
        return None;
    }
    let hash = zbase32(&Sha1::digest(local.to_lowercase().as_bytes()));
    let mut advanced = Url::parse(&format!("https://openpgpkey.{domain}/.well-known/openpgpkey/{domain}/hu/{hash}")).ok()?;
    advanced.query_pairs_mut().append_pair("l", local);
    let mut direct = Url::parse(&format!("https://{domain}/.well-known/openpgpkey/hu/{hash}")).ok()?;
    direct.query_pairs_mut().append_pair("l", local);
    Some([advanced, direct])
}

pub fn vks_url(email: &str) -> Option<Url> {
    let mut url = Url::parse("https://keys.openpgp.org/vks/v1/by-email/").ok()?;
    url.path_segments_mut().ok()?.pop_if_empty().push(email.trim());
    Some(url)
}

async fn fetch(http: &reqwest::Client, url: Url) -> Option<Vec<u8>> {
    let response = http.get(url).timeout(std::time::Duration::from_secs(10)).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let bytes = response.bytes().await.ok()?;
    (!bytes.is_empty() && bytes.len() <= MAX_KEY_BYTES).then(|| bytes.to_vec())
}

/// Procura a chave pública do e-mail. Devolve os bytes (binários ou em armadura) e de onde vieram.
pub async fn search(http: &reqwest::Client, email: &str) -> Result<Option<(Vec<u8>, &'static str)>, AppError> {
    if !email.contains('@') {
        return Err(AppError::InvalidInput);
    }
    if let Some([advanced, direct]) = wkd_urls(email) {
        for url in [advanced, direct] {
            if let Some(bytes) = fetch(http, url).await {
                tracing::info!(source = "wkd", "chave encontrada na internet");
                return Ok(Some((bytes, "wkd")));
            }
        }
    }
    if let Some(url) = vks_url(email) {
        if let Some(bytes) = fetch(http, url).await {
            tracing::info!(source = "keys.openpgp.org", "chave encontrada na internet");
            return Ok(Some((bytes, "keys.openpgp.org")));
        }
    }
    tracing::info!("chave não encontrada na internet");
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wkd_hash_matches_the_specification_example() {
        // Exemplo do draft-koch-openpgp-webkey-service: "Joe.Doe" → iy9q119eutrkn8s1mk4r39qejnbu3n5q.
        let [advanced, direct] = wkd_urls("Joe.Doe@Example.ORG").unwrap();
        assert_eq!(direct.as_str(), "https://example.org/.well-known/openpgpkey/hu/iy9q119eutrkn8s1mk4r39qejnbu3n5q?l=Joe.Doe");
        assert_eq!(advanced.as_str(), "https://openpgpkey.example.org/.well-known/openpgpkey/example.org/hu/iy9q119eutrkn8s1mk4r39qejnbu3n5q?l=Joe.Doe");
        assert!(wkd_urls("sem-arroba").is_none());
        assert!(wkd_urls("a@evil.com/x").is_none());
    }

    #[test]
    fn vks_url_escapes_the_address() {
        assert_eq!(vks_url("ana+news@x.com").unwrap().as_str(), "https://keys.openpgp.org/vks/v1/by-email/ana+news@x.com");
        assert_eq!(vks_url("a b@x.com").unwrap().as_str(), "https://keys.openpgp.org/vks/v1/by-email/a%20b@x.com");
    }
}
