//! Estado da conta em memória. O access token nunca sai do processo Rust nem vai para o keyring.

use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::{oneshot, Mutex};

use crate::error::AppError;
use crate::keyring_store::{self, StoredAccount};
use crate::oauth::{self, Tokens};
use crate::provider::Provider;

const REFRESH_MARGIN: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize)]
pub struct Account {
    pub provider: Provider,
    pub email: String,
}

struct AccessToken {
    value: String,
    expires_at: Instant,
}

struct Session {
    account: Account,
    refresh_token: String,
    access: Option<AccessToken>,
}

impl Session {
    fn from_stored(stored: StoredAccount) -> Self {
        Session {
            account: Account { provider: stored.provider, email: stored.email },
            refresh_token: stored.refresh_token,
            access: None,
        }
    }

    fn to_stored(&self) -> StoredAccount {
        StoredAccount {
            provider: self.account.provider,
            email: self.account.email.clone(),
            refresh_token: self.refresh_token.clone(),
        }
    }

    fn valid_access_token(&self) -> Option<&str> {
        self.access
            .as_ref()
            .filter(|a| a.expires_at > Instant::now() + REFRESH_MARGIN)
            .map(|a| a.value.as_str())
    }
}

fn access_from(tokens: &Tokens) -> AccessToken {
    AccessToken {
        value: tokens.access_token.clone(),
        expires_at: Instant::now() + Duration::from_secs(tokens.expires_in),
    }
}

pub struct AppState {
    pub http: reqwest::Client,
    session: Mutex<Option<Session>>,
    pending_connect: std::sync::Mutex<Option<oneshot::Sender<()>>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .expect("cliente HTTP"),
            session: Mutex::new(None),
            pending_connect: std::sync::Mutex::new(None),
        }
    }

    /// Conta atual: em memória ou, na primeira chamada, lida do keyring.
    pub async fn account(&self) -> Result<Option<Account>, AppError> {
        let mut guard = self.session.lock().await;
        if guard.is_none() {
            *guard = keyring_store::load()?.map(Session::from_stored);
        }
        Ok(guard.as_ref().map(|s| s.account.clone()))
    }

    /// Registra um novo fluxo OAuth; um fluxo anterior ainda pendente é cancelado.
    pub fn begin_connect(&self) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        let previous = self.pending_connect.lock().expect("mutex").replace(tx);
        if let Some(previous) = previous {
            let _ = previous.send(());
        }
        rx
    }

    pub fn cancel_connect(&self) {
        if let Some(tx) = self.pending_connect.lock().expect("mutex").take() {
            let _ = tx.send(());
        }
    }

    pub fn end_connect(&self) {
        self.pending_connect.lock().expect("mutex").take();
    }

    async fn replace_session(&self, session: Session) -> Result<Account, AppError> {
        keyring_store::save(&session.to_stored())?;
        let account = session.account.clone();
        tracing::info!(provider = ?account.provider, "conta conectada e salva no keyring");
        *self.session.lock().await = Some(session);
        Ok(account)
    }

    /// Conta recém-autorizada por OAuth, mantida em memória com o access token.
    pub async fn store_oauth_account(&self, provider: Provider, email: String, tokens: Tokens) -> Result<Account, AppError> {
        let refresh_token = tokens.refresh_token.clone().ok_or_else(|| {
            tracing::error!(?provider, "provedor não devolveu refresh_token; confira access_type/offline_access");
            AppError::AuthFailed
        })?;
        self.replace_session(Session {
            account: Account { provider, email },
            refresh_token,
            access: Some(access_from(&tokens)),
        })
        .await
    }

    /// Devolve a conta e um access token válido, renovando pelo refresh token quando necessário.
    /// `invalid_grant` apaga a conta do keyring e devolve `TokenExpired`.
    pub async fn access(&self, force_refresh: bool) -> Result<(Account, String), AppError> {
        let mut guard = self.session.lock().await;
        if guard.is_none() {
            *guard = keyring_store::load()?.map(Session::from_stored);
        }
        let session = guard.as_mut().ok_or(AppError::TokenExpired)?;
        if !force_refresh {
            if let Some(token) = session.valid_access_token() {
                return Ok((session.account.clone(), token.to_owned()));
            }
        }

        let provider = session.account.provider;
        match oauth::refresh(&self.http, provider, &session.refresh_token).await {
            Ok(tokens) => {
                session.access = Some(access_from(&tokens));
                if let Some(rotated) = tokens.refresh_token.filter(|t| *t != session.refresh_token) {
                    session.refresh_token = rotated;
                    keyring_store::save(&session.to_stored())?;
                }
                tracing::info!(?provider, "access token renovado");
                Ok((session.account.clone(), tokens.access_token))
            }
            Err(AppError::TokenExpired) => {
                *guard = None;
                keyring_store::delete()?;
                Err(AppError::TokenExpired)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn disconnect(&self) -> Result<(), AppError> {
        *self.session.lock().await = None;
        keyring_store::delete()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
