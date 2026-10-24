//! OAuth 2.0 para apps nativos (RFC 8252): PKCE S256 + redirect de loopback em porta efêmera.
//! Nenhum token, código ou `state` é registrado em log.

use std::collections::HashMap;
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use reqwest::Url;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::error::AppError;
use crate::provider::{OAuthClient, Provider};

const CALLBACK_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const MAX_REQUEST_BYTES: usize = 16 * 1024;

const CALLBACK_PAGE: &str = "<!doctype html><html lang=\"pt-BR\"><meta charset=\"utf-8\"><title>Epistula</title>\
<body style=\"font-family:system-ui,sans-serif;display:grid;place-items:center;height:100vh;margin:0\">\
<p>Pronto. Você já pode fechar esta aba e voltar para o Epistula.</p></body></html>";

pub struct Tokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    pub id_token: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    id_token: Option<String>,
}

#[derive(Deserialize)]
struct TokenErrorResponse {
    error: String,
}

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

pub fn pkce() -> Pkce {
    let verifier = random_token(32);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    Pkce { verifier, challenge }
}

pub fn new_state() -> String {
    random_token(16)
}

pub fn authorize_url(provider: Provider, client_id: &str, redirect_uri: &str, challenge: &str, state: &str) -> Url {
    let mut url = Url::parse(provider.auth_url()).expect("URL de autorização fixa e válida");
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("client_id", client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", provider.scopes())
            .append_pair("code_challenge", challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", state);
        for (k, v) in provider.extra_auth_params() {
            q.append_pair(k, v);
        }
    }
    url
}

/// Executa o fluxo completo: abre o navegador, espera o callback e troca o código por tokens.
/// Retorna os tokens e o e-mail da conta (das claims do `id_token`).
pub async fn authorize(
    http: &reqwest::Client,
    provider: Provider,
    cancel: oneshot::Receiver<()>,
    open_browser: impl FnOnce(&str) -> Result<(), AppError>,
) -> Result<(Tokens, String), AppError> {
    let client = provider.client()?;
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.map_err(|e| {
        tracing::error!(error = %e, "não foi possível abrir o listener de loopback");
        AppError::AuthFailed
    })?;
    let port = listener.local_addr().map_err(|_| AppError::AuthFailed)?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}");

    let pkce = pkce();
    let state = new_state();
    open_browser(authorize_url(provider, &client.client_id, &redirect_uri, &pkce.challenge, &state).as_str())?;
    tracing::info!(?provider, port, "aguardando callback OAuth");

    let params = tokio::select! {
        result = tokio::time::timeout(CALLBACK_TIMEOUT, wait_for_callback(&listener)) => match result {
            Ok(params) => params?,
            Err(_) => return Err(AppError::AuthTimeout),
        },
        _ = cancel => return Err(AppError::AuthCancelled),
    };
    drop(listener);

    let code = validate_callback(&params, &state)?;
    let tokens = exchange_code(http, provider, &client, &code, &pkce.verifier, &redirect_uri).await?;
    let email = tokens.id_token.as_deref().and_then(email_from_id_token).ok_or_else(|| {
        tracing::error!(?provider, "id_token sem claim de e-mail");
        AppError::AuthFailed
    })?;
    Ok((tokens, email))
}

/// Valida o `state` e extrai o `code`; `error=access_denied` significa que o usuário recusou.
pub fn validate_callback(params: &HashMap<String, String>, expected_state: &str) -> Result<String, AppError> {
    if params.get("state").map(String::as_str) != Some(expected_state) {
        tracing::warn!("callback OAuth com state inválido");
        return Err(AppError::AuthFailed);
    }
    if let Some(error) = params.get("error") {
        tracing::info!(error = %error, "provedor devolveu erro no callback");
        return Err(if error == "access_denied" { AppError::AuthCancelled } else { AppError::AuthFailed });
    }
    params.get("code").filter(|c| !c.is_empty()).cloned().ok_or(AppError::AuthFailed)
}

/// Aceita conexões até chegar o GET em `/`; outras rotas (ex.: /favicon.ico) recebem 404.
async fn wait_for_callback(listener: &TcpListener) -> Result<HashMap<String, String>, AppError> {
    loop {
        let (mut stream, _) = listener.accept().await.map_err(|_| AppError::AuthFailed)?;
        let mut buf = Vec::with_capacity(1024);
        let mut chunk = [0u8; 1024];
        while !buf.windows(4).any(|w| w == b"\r\n\r\n") && buf.len() < MAX_REQUEST_BYTES {
            match stream.read(&mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
            }
        }
        let request = String::from_utf8_lossy(&buf);
        let target = request
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("GET "))
            .and_then(|rest| rest.split(' ').next())
            .unwrap_or("");
        let parsed = Url::parse(&format!("http://127.0.0.1{target}")).ok();
        match parsed {
            Some(url) if url.path() == "/" && url.query().is_some() => {
                let body = CALLBACK_PAGE;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.shutdown().await;
                return Ok(url.query_pairs().into_owned().collect());
            }
            _ => {
                let _ = stream
                    .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .await;
            }
        }
    }
}

async fn exchange_code(
    http: &reqwest::Client,
    provider: Provider,
    client: &OAuthClient,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<Tokens, AppError> {
    let mut form = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", client.client_id.as_str()),
        ("code_verifier", verifier),
    ];
    if let Some(secret) = &client.client_secret {
        form.push(("client_secret", secret.as_str()));
    }
    if provider.sends_scope_to_token_endpoint() {
        form.push(("scope", provider.scopes()));
    }
    token_request(http, provider, &form, AppError::AuthFailed).await
}

/// Renova o access token. `invalid_grant` (refresh revogado ou expirado) vira `TokenExpired`.
pub async fn refresh(http: &reqwest::Client, provider: Provider, refresh_token: &str) -> Result<Tokens, AppError> {
    let client = provider.client()?;
    let mut form = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client.client_id.as_str()),
    ];
    if let Some(secret) = &client.client_secret {
        form.push(("client_secret", secret.as_str()));
    }
    if provider.sends_scope_to_token_endpoint() {
        form.push(("scope", provider.scopes()));
    }
    token_request(http, provider, &form, AppError::ImapConnection).await
}

/// Revoga o refresh token no provedor (sair da conta). Melhor esforço: quem chama segue em frente
/// se falhar. A Microsoft não tem endpoint público de revogação para apps nativos.
pub async fn revoke(http: &reqwest::Client, provider: Provider, refresh_token: &str) -> Result<(), AppError> {
    let Some(url) = provider.revoke_url() else { return Ok(()) };
    let response = http.post(url).form(&[("token", refresh_token)]).send().await.map_err(|e| {
        tracing::warn!(?provider, error = %e.without_url(), "falha de rede ao revogar o token");
        AppError::ImapConnection
    })?;
    if !response.status().is_success() {
        tracing::warn!(?provider, status = %response.status(), "provedor recusou a revogação");
        return Err(AppError::AuthFailed);
    }
    tracing::info!(?provider, "refresh token revogado no provedor");
    Ok(())
}

/// `offline` é o erro devolvido quando a rede falha: no login é `AuthFailed`; na renovação em
/// segundo plano é `ImapConnection`, para o app tratar como "sem conexão" e não como acesso negado.
async fn token_request(http: &reqwest::Client, provider: Provider, form: &[(&str, &str)], offline: AppError) -> Result<Tokens, AppError> {
    let response = http.post(provider.token_url()).form(form).send().await.map_err(|e| {
        tracing::warn!(?provider, error = %e.without_url(), "falha de rede no endpoint de token");
        offline
    })?;
    let status = response.status();
    if !status.is_success() {
        let error = response.json::<TokenErrorResponse>().await.map(|e| e.error).unwrap_or_default();
        tracing::warn!(?provider, %status, error = %error, "endpoint de token recusou a requisição");
        return Err(if error == "invalid_grant" { AppError::TokenExpired } else { AppError::AuthFailed });
    }
    let body: TokenResponse = response.json().await.map_err(|_| AppError::AuthFailed)?;
    Ok(Tokens {
        access_token: body.access_token,
        refresh_token: body.refresh_token,
        expires_in: body.expires_in.unwrap_or(3600),
        id_token: body.id_token,
    })
}

/// Lê o e-mail das claims do `id_token`. A assinatura não é verificada: o token veio direto
/// do endpoint de token do provedor por TLS (OpenID Connect Core, seção 3.1.3.7).
pub fn email_from_id_token(id_token: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Claims {
        email: Option<String>,
        preferred_username: Option<String>,
    }
    let payload = id_token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    let claims: Claims = serde_json::from_slice(&bytes).ok()?;
    claims
        .email
        .or(claims.preferred_username)
        .filter(|e| e.contains('@'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_s256_of_verifier() {
        let p = pkce();
        assert_eq!(p.verifier.len(), 43);
        assert_eq!(p.challenge, URL_SAFE_NO_PAD.encode(Sha256::digest(p.verifier.as_bytes())));
    }

    #[test]
    fn pkce_matches_rfc7636_example() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn states_are_unique() {
        assert_ne!(new_state(), new_state());
    }

    #[test]
    fn authorize_url_has_pkce_and_offline_params() {
        let url = authorize_url(Provider::Google, "cid", "http://127.0.0.1:5000", "chal", "st");
        let q: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["code_challenge"], "chal");
        assert_eq!(q["state"], "st");
        assert_eq!(q["redirect_uri"], "http://127.0.0.1:5000");
        assert_eq!(q["access_type"], "offline");
        assert!(q["scope"].contains("https://mail.google.com/"));
    }

    fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn callback_rejects_wrong_state() {
        assert_eq!(validate_callback(&params(&[("state", "x"), ("code", "c")]), "y"), Err(AppError::AuthFailed));
    }

    #[test]
    fn callback_maps_access_denied_to_cancelled() {
        let p = params(&[("state", "s"), ("error", "access_denied")]);
        assert_eq!(validate_callback(&p, "s"), Err(AppError::AuthCancelled));
    }

    #[test]
    fn callback_returns_code() {
        assert_eq!(validate_callback(&params(&[("state", "s"), ("code", "abc")]), "s"), Ok("abc".into()));
    }

    #[test]
    fn email_claim_is_read_from_id_token() {
        let payload = URL_SAFE_NO_PAD.encode(br#"{"email":"ana@gmail.com"}"#);
        assert_eq!(email_from_id_token(&format!("h.{payload}.s")), Some("ana@gmail.com".into()));
        let payload = URL_SAFE_NO_PAD.encode(br#"{"preferred_username":"bia@outlook.com"}"#);
        assert_eq!(email_from_id_token(&format!("h.{payload}.s")), Some("bia@outlook.com".into()));
        assert_eq!(email_from_id_token("lixo"), None);
    }

    #[tokio::test]
    async fn loopback_ignores_favicon_and_returns_query() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move { wait_for_callback(&listener).await });
        for path in ["/favicon.ico", "/?code=abc&state=xyz"] {
            let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            s.write_all(format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes()).await.unwrap();
            let mut out = Vec::new();
            s.read_to_end(&mut out).await.unwrap();
        }
        let q = server.await.unwrap().unwrap();
        assert_eq!(q["code"], "abc");
        assert_eq!(q["state"], "xyz");
    }
}
