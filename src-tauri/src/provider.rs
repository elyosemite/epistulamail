use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Google,
    Microsoft,
}

pub struct OAuthClient {
    pub client_id: String,
    pub client_secret: Option<String>,
}

/// Endereço de callback do login. O Google aceita a porta efêmera em `127.0.0.1`; a Microsoft
/// só ignora a porta quando o host é `localhost`, então o app usa uma porta fixa registrada no Entra.
pub struct RedirectUri {
    pub host: &'static str,
    pub port: Option<u16>,
    pub path: &'static str,
}

pub const MICROSOFT_REDIRECT_PORT: u16 = 8400;
pub const MICROSOFT_REDIRECT_PATH: &str = "/oauth/callback";

impl Provider {
    pub fn auth_url(self) -> &'static str {
        match self {
            Provider::Google => "https://accounts.google.com/o/oauth2/v2/auth",
            Provider::Microsoft => "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize",
        }
    }

    pub fn token_url(self) -> &'static str {
        match self {
            Provider::Google => "https://oauth2.googleapis.com/token",
            Provider::Microsoft => "https://login.microsoftonline.com/consumers/oauth2/v2.0/token",
        }
    }

    pub fn redirect(self) -> RedirectUri {
        match self {
            Provider::Google => RedirectUri { host: "127.0.0.1", port: None, path: "/" },
            Provider::Microsoft => RedirectUri {
                host: "localhost",
                port: Some(MICROSOFT_REDIRECT_PORT),
                path: MICROSOFT_REDIRECT_PATH,
            },
        }
    }

    pub fn revoke_url(self) -> Option<&'static str> {
        match self {
            Provider::Google => Some("https://oauth2.googleapis.com/revoke"),
            Provider::Microsoft => None,
        }
    }

    pub fn scopes(self) -> &'static str {
        match self {
            Provider::Google => "https://mail.google.com/ openid email profile",
            Provider::Microsoft => "openid profile email offline_access User.Read Mail.ReadWrite Mail.Send",
        }
    }

    /// Parâmetros extras da URL de autorização. No Google, `access_type=offline` e
    /// `prompt=consent` garantem que o `refresh_token` venha em toda conexão.
    pub fn extra_auth_params(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Provider::Google => &[("access_type", "offline"), ("prompt", "consent")],
            Provider::Microsoft => &[("prompt", "select_account")],
        }
    }

    /// A Microsoft quer o escopo também na troca de código e no refresh; o Google não.
    pub fn sends_scope_to_token_endpoint(self) -> bool {
        matches!(self, Provider::Microsoft)
    }

    pub fn client(self) -> Result<OAuthClient, AppError> {
        let (id, secret) = match self {
            Provider::Google => (
                setting("EPISTULA_GOOGLE_CLIENT_ID", option_env!("EPISTULA_GOOGLE_CLIENT_ID")),
                setting("EPISTULA_GOOGLE_CLIENT_SECRET", option_env!("EPISTULA_GOOGLE_CLIENT_SECRET")),
            ),
            Provider::Microsoft => (setting("EPISTULA_MS_CLIENT_ID", option_env!("EPISTULA_MS_CLIENT_ID")), None),
        };
        let client_id = id.ok_or_else(|| {
            tracing::error!(provider = ?self, "client_id OAuth ausente; configure as variáveis EPISTULA_*");
            AppError::Config
        })?;
        Ok(OAuthClient { client_id, client_secret: secret })
    }
}

/// Lê a configuração do ambiente em tempo de execução e, se ausente, do valor embutido no build.
fn setting(name: &str, compiled: Option<&'static str>) -> Option<String> {
    std::env::var(name)
        .ok()
        .or_else(|| compiled.map(str::to_owned))
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn microsoft_uses_consumers_and_the_registered_redirect() {
        assert!(Provider::Microsoft.auth_url().contains("/consumers/oauth2/v2.0/authorize"));
        assert!(Provider::Microsoft.token_url().contains("/consumers/oauth2/v2.0/token"));
        assert_eq!(
            Provider::Microsoft.scopes(),
            "openid profile email offline_access User.Read Mail.ReadWrite Mail.Send"
        );
        let redirect = Provider::Microsoft.redirect();
        assert_eq!(redirect.host, "localhost");
        assert_eq!(redirect.port, Some(MICROSOFT_REDIRECT_PORT));
        assert_eq!(redirect.path, MICROSOFT_REDIRECT_PATH);
    }

    #[test]
    fn google_keeps_the_ephemeral_loopback_redirect() {
        let redirect = Provider::Google.redirect();
        assert_eq!(redirect.host, "127.0.0.1");
        assert_eq!(redirect.port, None);
        assert_eq!(redirect.path, "/");
    }
}
