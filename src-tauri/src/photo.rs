//! Foto do perfil da conta (US5, CA5). A URL vem da claim `picture` do `id_token`; o processo Rust
//! baixa a imagem e a guarda no banco cifrado. A WebView só recebe uma `data:` URL, então nenhuma
//! requisição sai dela para o provedor. A URL e os bytes nunca vão para o log.

use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use reqwest::Url;
use serde::Deserialize;

use crate::error::AppError;

pub const MAX_PHOTO_BYTES: usize = 256 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);
const ALLOWED_TYPES: &[&str] = &["image/jpeg", "image/png", "image/webp"];

/// Lê a claim `picture` do `id_token` (mesma premissa de `oauth::email_from_id_token`: o token
/// veio direto do endpoint de token por TLS).
pub fn picture_from_id_token(id_token: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Claims {
        picture: Option<String>,
    }
    let payload = id_token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    serde_json::from_slice::<Claims>(&bytes).ok()?.picture
}

/// Nome do titular (claim `name`), para o cabeçalho `From` do que for enviado (US7). Sem
/// controles e com no máximo 100 caracteres.
pub fn name_from_id_token(id_token: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Claims {
        name: Option<String>,
    }
    let payload = id_token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    let name = serde_json::from_slice::<Claims>(&bytes).ok()?.name?;
    let clean: String = name.chars().filter(|c| !c.is_control()).take(100).collect();
    Some(clean.trim().to_owned()).filter(|n| !n.is_empty())
}

/// Só `https` em `googleusercontent.com` (onde o Google serve as fotos de perfil).
pub fn allowed_url(url: &str) -> Option<Url> {
    let parsed = Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    let google = host == "googleusercontent.com" || host.ends_with(".googleusercontent.com");
    (parsed.scheme() == "https" && google && parsed.username().is_empty() && parsed.port().is_none()).then_some(parsed)
}

/// Tipo aceito a partir do `Content-Type` (sem parâmetros).
pub fn allowed_type(content_type: &str) -> Option<&'static str> {
    let essence = content_type.split(';').next()?.trim().to_ascii_lowercase();
    ALLOWED_TYPES.iter().find(|t| **t == essence).copied()
}

/// Baixa a foto com limite de tamanho e tempo. Redirecionamentos são seguidos só se o destino
/// continuar permitido.
pub async fn download(http: &reqwest::Client, url: &str) -> Result<(Vec<u8>, &'static str), AppError> {
    let url = allowed_url(url).ok_or(AppError::InvalidInput)?;
    let mut response = http.get(url).timeout(TIMEOUT).send().await.map_err(|e| {
        tracing::warn!(error = %e.without_url(), "falha de rede ao baixar a foto da conta");
        AppError::ImapConnection
    })?;
    if !response.status().is_success() || allowed_url(response.url().as_str()).is_none() {
        tracing::warn!(status = %response.status(), "foto da conta recusada pelo servidor");
        return Err(AppError::InvalidInput);
    }
    let kind = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(allowed_type)
        .ok_or(AppError::InvalidInput)?;
    if response.content_length().is_some_and(|n| n as usize > MAX_PHOTO_BYTES) {
        return Err(AppError::MessageTooLarge);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| AppError::ImapConnection)? {
        if bytes.len() + chunk.len() > MAX_PHOTO_BYTES {
            return Err(AppError::MessageTooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Err(AppError::InvalidInput);
    }
    tracing::info!(bytes = bytes.len(), kind, "foto da conta baixada");
    Ok((bytes, kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(claims: &str) -> String {
        format!("h.{}.s", URL_SAFE_NO_PAD.encode(claims))
    }

    #[test]
    fn picture_claim_is_read_from_id_token() {
        let t = token(r#"{"email":"ana@gmail.com","picture":"https://lh3.googleusercontent.com/a/x=s96-c"}"#);
        assert_eq!(picture_from_id_token(&t).as_deref(), Some("https://lh3.googleusercontent.com/a/x=s96-c"));
        assert_eq!(picture_from_id_token(&token(r#"{"email":"ana@gmail.com"}"#)), None);
        assert_eq!(picture_from_id_token("lixo"), None);
    }

    #[test]
    fn only_https_googleusercontent_is_allowed() {
        assert!(allowed_url("https://lh3.googleusercontent.com/a/x=s96-c").is_some());
        for bad in [
            "http://lh3.googleusercontent.com/a/x",
            "https://googleusercontent.com.evil.com/a",
            "https://evilgoogleusercontent.com/a",
            "https://user@lh3.googleusercontent.com/a",
            "https://lh3.googleusercontent.com:8443/a",
            "file:///C:/foto.jpg",
            "data:image/png;base64,AAAA",
            "",
        ] {
            assert!(allowed_url(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn only_raster_image_types_are_allowed() {
        assert_eq!(allowed_type("image/jpeg"), Some("image/jpeg"));
        assert_eq!(allowed_type("IMAGE/PNG; charset=binary"), Some("image/png"));
        assert_eq!(allowed_type("image/webp"), Some("image/webp"));
        for bad in ["image/svg+xml", "text/html", "application/octet-stream", ""] {
            assert_eq!(allowed_type(bad), None, "{bad}");
        }
    }
}
