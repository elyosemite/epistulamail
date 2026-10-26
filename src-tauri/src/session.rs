//! Estado em memória: banco, sessões por conta e o fluxo de login em andamento. O access token de
//! cada conta nunca sai do processo Rust nem vai para o keyring ou o banco.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{oneshot, Mutex};

use crate::db::Db;
use crate::error::AppError;
use crate::imap_client::ImapSession;
use crate::oauth::{self, Tokens};
use crate::photo;
use crate::provider::Provider;
use crate::providers::{self, AuthKind};
use crate::secrets;
use crate::store::{self, Account, AccountStatus};

const REFRESH_MARGIN: Duration = Duration::from_secs(60);

struct AccessToken {
    value: String,
    expires_at: Instant,
}

fn access_from(tokens: &Tokens) -> AccessToken {
    AccessToken {
        value: tokens.access_token.clone(),
        expires_at: Instant::now() + Duration::from_secs(tokens.expires_in),
    }
}

#[derive(Default)]
struct Auth {
    refresh_token: Option<String>,
    access: Option<AccessToken>,
}

/// Qual das duas conexões IMAP da conta usar (CA3: no máximo 2 por conta). A interativa atende o
/// usuário (abrir mensagem, mudar flag); a de sincronização roda em segundo plano sem bloqueá-lo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    Interactive,
    Sync,
}

pub struct AccountHandle {
    pub id: String,
    pub provider: Provider,
    pub email: String,
    pub imap_host: String,
    pub imap_port: u16,
    auth: Mutex<Auth>,
    interactive: Mutex<Option<ImapSession>>,
    sync: Mutex<Option<ImapSession>>,
    /// Pastas com sincronização em andamento (uma por pasta por vez).
    pub syncing: std::sync::Mutex<HashSet<i64>>,
}

impl AccountHandle {
    pub fn lane(&self, lane: Lane) -> &Mutex<Option<ImapSession>> {
        match lane {
            Lane::Interactive => &self.interactive,
            Lane::Sync => &self.sync,
        }
    }

    pub async fn close_sessions(&self) {
        for lane in [Lane::Interactive, Lane::Sync] {
            if let Some(mut session) = self.lane(lane).lock().await.take() {
                let _ = tokio::time::timeout(Duration::from_secs(3), session.logout()).await;
            }
        }
    }

    pub fn try_begin_sync(&self, folder_id: i64) -> bool {
        self.syncing.lock().map(|mut s| s.insert(folder_id)).unwrap_or(false)
    }

    pub fn end_sync(&self, folder_id: i64) {
        if let Ok(mut s) = self.syncing.lock() {
            s.remove(&folder_id);
        }
    }

    pub fn is_syncing(&self, folder_id: i64) -> bool {
        self.syncing.lock().map(|s| s.contains(&folder_id)).unwrap_or(false)
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountEvent {
    pub account_id: String,
}

pub struct AppState {
    pub app: AppHandle,
    pub http: reqwest::Client,
    pub db: Db,
    accounts: Mutex<HashMap<String, Arc<AccountHandle>>>,
    pending_connect: std::sync::Mutex<Option<oneshot::Sender<()>>>,
    /// Inicialização (índice de contas, migração, limpeza) concluída. A janela abre antes dela; só
    /// a lista de contas espera.
    started: tokio::sync::watch::Sender<bool>,
    /// O frontend já mostrou a primeira tela: a sincronização automática pode começar.
    ui_ready: tokio::sync::Notify,
}

impl AppState {
    pub fn new(app: AppHandle, db: Db) -> Result<Self, AppError> {
        let http = reqwest::Client::builder().timeout(Duration::from_secs(20)).build().map_err(|e| {
            tracing::error!(error = %e, "não foi possível criar o cliente HTTP");
            AppError::Config
        })?;
        Ok(AppState {
            app,
            http,
            db,
            accounts: Mutex::new(HashMap::new()),
            pending_connect: std::sync::Mutex::new(None),
            started: tokio::sync::watch::channel(false).0,
            ui_ready: tokio::sync::Notify::new(),
        })
    }

    pub fn mark_started(&self) {
        self.started.send_replace(true);
    }

    pub async fn wait_started(&self) {
        let mut rx = self.started.subscribe();
        let _ = rx.wait_for(|done| *done).await;
    }

    /// Chamado pelo frontend depois da primeira tela (`app_ready`).
    pub fn mark_ui_ready(&self) {
        self.ui_ready.notify_one();
    }

    /// Espera a primeira tela, com limite: sem resposta do frontend, a sincronização começa assim mesmo.
    pub async fn wait_ui_ready(&self, limit: Duration) {
        let _ = tokio::time::timeout(limit, self.ui_ready.notified()).await;
    }

    pub fn emit_accounts_changed(&self) {
        let _ = self.app.emit("accounts-updated", ());
        // Tooltip da bandeja e ponto na barra de tarefas acompanham as não lidas (US8).
        crate::tray::refresh(&self.app);
    }

    /// Sessão em memória da conta, criada na primeira vez a partir do banco.
    pub async fn handle(&self, account_id: &str) -> Result<Arc<AccountHandle>, AppError> {
        if let Some(h) = self.accounts.lock().await.get(account_id) {
            return Ok(h.clone());
        }
        let id = account_id.to_owned();
        let (account, (host, port)) = self
            .db
            .call(move |c| Ok((store::require_account(c, &id)?, store::imap_server(c, &id)?)))
            .await?;
        let provider = match providers::find(&account.provider).map(|p| p.auth) {
            Some(AuthKind::OAuth2(p)) => p,
            _ => return Err(AppError::ProviderUnavailable),
        };
        let handle = Arc::new(AccountHandle {
            id: account.id.clone(),
            provider,
            email: account.email,
            imap_host: host,
            imap_port: port,
            auth: Mutex::new(Auth::default()),
            interactive: Mutex::new(None),
            sync: Mutex::new(None),
            syncing: std::sync::Mutex::new(HashSet::new()),
        });
        let mut map = self.accounts.lock().await;
        Ok(map.entry(account.id).or_insert(handle).clone())
    }

    /// Esquece a sessão em memória (tokens e conexões) de uma conta.
    pub async fn drop_handle(&self, account_id: &str) {
        let handle = self.accounts.lock().await.remove(account_id);
        if let Some(handle) = handle {
            handle.close_sessions().await;
        }
    }

    pub async fn close_all(&self) {
        let handles: Vec<_> = self.accounts.lock().await.values().cloned().collect();
        for h in handles {
            h.close_sessions().await;
        }
    }

    async fn status_of(&self, account_id: &str) -> Result<AccountStatus, AppError> {
        let id = account_id.to_owned();
        self.db.call(move |c| Ok(store::require_account(c, &id)?.status)).await
    }

    /// Conta perdeu o acesso (refresh revogado, expirado ou ausente): fica `needs_reauth`, sem
    /// segredo no keyring, e o frontend é avisado.
    async fn lose_access(&self, handle: &AccountHandle) {
        let _ = secrets::delete_refresh_token(&handle.id);
        let id = handle.id.clone();
        let _ = self.db.call(move |c| store::set_status(c, &id, AccountStatus::NeedsReauth)).await;
        let _ = self.app.emit("account-status", AccountEvent { account_id: handle.id.clone() });
        self.emit_accounts_changed();
        let id = handle.id.clone();
        if let Ok(account) = self.db.call(move |c| store::require_account(c, &id)).await {
            crate::notify::reauth(&self.app, &account.id, &account.display_name, &account.email);
        }
    }

    /// Access token válido da conta, renovando pelo refresh token quando necessário.
    /// Chamado por `imap_client::with_session` com o lock de uma conexão tomado: não usa conexões.
    pub async fn access(&self, handle: &AccountHandle, force_refresh: bool) -> Result<String, AppError> {
        match self.status_of(&handle.id).await? {
            AccountStatus::Active => {}
            AccountStatus::SignedOut => return Err(AppError::SignedOut),
            AccountStatus::NeedsReauth => return Err(AppError::TokenExpired),
        }
        let mut auth = handle.auth.lock().await;
        if !force_refresh {
            if let Some(a) = auth.access.as_ref().filter(|a| a.expires_at > Instant::now() + REFRESH_MARGIN) {
                return Ok(a.value.clone());
            }
        }
        if auth.refresh_token.is_none() {
            auth.refresh_token = secrets::load_refresh_token(&handle.id)?;
        }
        let Some(refresh_token) = auth.refresh_token.clone() else {
            drop(auth);
            tracing::warn!("conta ativa sem refresh token no keyring");
            self.lose_access(handle).await;
            return Err(AppError::TokenExpired);
        };
        match oauth::refresh(&self.http, handle.provider, &refresh_token).await {
            Ok(tokens) => {
                auth.access = Some(access_from(&tokens));
                if let Some(rotated) = tokens.refresh_token.filter(|t| *t != refresh_token) {
                    secrets::save_refresh_token(&handle.id, &rotated)?;
                    auth.refresh_token = Some(rotated);
                }
                tracing::info!(provider = ?handle.provider, "access token renovado");
                self.update_photo(&handle.id, tokens.id_token.as_deref(), false);
                Ok(tokens.access_token)
            }
            Err(AppError::TokenExpired) => {
                auth.refresh_token = None;
                auth.access = None;
                drop(auth);
                self.lose_access(handle).await;
                Err(AppError::TokenExpired)
            }
            Err(e) => Err(e),
        }
    }

    /// Baixa a foto do perfil em segundo plano (US5, CA5). `force` ignora o limite de uma vez por
    /// dia (conectar e reconectar). Sem a claim `picture` (escopo `profile` não autorizado), a foto
    /// salva continua; sem rede, tenta de novo na próxima renovação.
    pub fn update_photo(&self, account_id: &str, id_token: Option<&str>, force: bool) {
        // O nome do titular vem no mesmo token (US7: cabeçalho From do envio).
        if let Some(name) = id_token.and_then(photo::name_from_id_token) {
            let (db, id) = (self.db.clone(), account_id.to_owned());
            tauri::async_runtime::spawn(async move {
                let _ = db.call(move |c| store::set_sender_name(c, &id, Some(&name))).await;
            });
        }
        let Some(url) = id_token.and_then(photo::picture_from_id_token) else { return };
        let app = self.app.clone();
        let id = account_id.to_owned();
        tauri::async_runtime::spawn(async move {
            let state = app.state::<AppState>();
            if !force {
                let account = id.clone();
                if !matches!(state.db.call(move |c| store::photo_due(c, &account)).await, Ok(true)) {
                    return;
                }
            }
            let photo = match photo::download(&state.http, &url).await {
                Ok(photo) => Some(photo),
                Err(AppError::ImapConnection) => return,
                Err(_) => None,
            };
            let account = id.clone();
            let saved = state.db.call(move |c| store::set_photo(c, &account, photo.as_ref().map(|(b, k)| (b.as_slice(), *k)))).await;
            if saved.is_ok() {
                state.emit_accounts_changed();
            }
        });
    }

    // ------------------------------------------------------------ login

    /// Registra um novo fluxo OAuth; um fluxo anterior ainda pendente é cancelado.
    pub fn begin_connect(&self) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        let previous = self.pending_connect.lock().ok().and_then(|mut p| p.replace(tx));
        if let Some(previous) = previous {
            let _ = previous.send(());
        }
        rx
    }

    pub fn cancel_connect(&self) {
        if let Some(tx) = self.pending_connect.lock().ok().and_then(|mut p| p.take()) {
            let _ = tx.send(());
        }
    }

    pub fn end_connect(&self) {
        if let Ok(mut p) = self.pending_connect.lock() {
            p.take();
        }
    }

    /// Grava a conta recém-autorizada. Mesma conta (provedor + e-mail) é reaproveitada, inclusive
    /// depois de sair dela; conta nova recebe nome e cor padrão.
    pub async fn store_oauth_account(&self, provider_id: &'static str, email: String, tokens: Tokens) -> Result<Connected, AppError> {
        let refresh_token = tokens.refresh_token.clone().ok_or_else(|| {
            tracing::error!(provider = provider_id, "provedor não devolveu refresh_token; confira access_type/offline_access");
            AppError::AuthFailed
        })?;
        let lookup_email = email.clone();
        let existing = self.db.call(move |c| store::find_account(c, provider_id, &lookup_email)).await?;
        let (account, is_new, was_active) = match existing {
            Some(account) => {
                secrets::save_refresh_token(&account.id, &refresh_token)?;
                let id = account.id.clone();
                let account_after = self
                    .db
                    .call(move |c| {
                        store::set_status(c, &id, AccountStatus::Active)?;
                        store::require_account(c, &id)
                    })
                    .await?;
                (account_after, false, account.status == AccountStatus::Active)
            }
            None => {
                let entry = providers::connectable(provider_id)?;
                let id = store::new_account_id();
                secrets::save_refresh_token(&id, &refresh_token)?;
                let account_id = id.clone();
                let created = self
                    .db
                    .call(move |c| {
                        let color = store::next_color(c)?;
                        store::insert_account(c, &account_id, entry, &email, &store::default_display_name(&email), color)
                    })
                    .await;
                match created {
                    Ok(account) => (account, true, false),
                    Err(e) => {
                        let _ = secrets::delete_refresh_token(&id);
                        return Err(e);
                    }
                }
            }
        };
        self.drop_handle(&account.id).await;
        let handle = self.handle(&account.id).await?;
        {
            let mut auth = handle.auth.lock().await;
            auth.refresh_token = Some(refresh_token);
            auth.access = Some(access_from(&tokens));
        }
        self.save_index().await;
        self.update_photo(&account.id, tokens.id_token.as_deref(), true);
        tracing::info!(provider = provider_id, is_new, "conta conectada");
        self.emit_accounts_changed();
        Ok(Connected { account, is_new, was_active })
    }

    /// Sair da conta (CA6): encerra conexões, revoga o token no provedor (melhor esforço), apaga o
    /// segredo do keyring e marca a conta como `signed_out`. Os dados do banco ficam (D4).
    pub async fn sign_out(&self, account_id: &str) -> Result<Account, AppError> {
        let handle = self.handle(account_id).await?;
        handle.close_sessions().await;
        let refresh = {
            let mut auth = handle.auth.lock().await;
            auth.access = None;
            auth.refresh_token.take()
        };
        let refresh = match refresh {
            Some(t) => Some(t),
            None => secrets::load_refresh_token(account_id).unwrap_or(None),
        };
        if let Some(token) = refresh {
            if let Err(e) = oauth::revoke(&self.http, handle.provider, &token).await {
                tracing::warn!(kind = e.kind(), "revogação não confirmada; seguindo com a saída");
            }
        }
        let id = account_id.to_owned();
        let account = self
            .db
            .call(move |c| {
                store::set_status(c, &id, AccountStatus::SignedOut)?;
                store::require_account(c, &id)
            })
            .await?;
        if let Err(e) = secrets::delete_refresh_token(account_id) {
            // A conta já está `signed_out`; a remoção é tentada de novo na próxima abertura.
            tracing::warn!(kind = e.kind(), "segredo não removido agora; nova tentativa ao abrir o app");
        }
        self.drop_handle(account_id).await;
        self.emit_accounts_changed();
        tracing::info!("conta saiu; dados locais mantidos");
        Ok(account)
    }

    /// Atualiza o índice de contas no keyring (usado para recuperar contas se o banco for recriado).
    pub async fn save_index(&self) {
        let accounts = self.db.call(|c| store::list_accounts(c)).await.unwrap_or_default();
        let index: Vec<secrets::IndexEntry> = accounts
            .into_iter()
            .map(|a| secrets::IndexEntry { id: a.id, provider: a.provider, email: a.email })
            .collect();
        if let Err(e) = secrets::save_index(&index) {
            tracing::warn!(kind = e.kind(), "índice de contas não atualizado");
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connected {
    pub account: Account,
    pub is_new: bool,
    /// A conta já estava conectada e ativa: o frontend avisa "Esta conta já está no Epistula".
    pub was_active: bool,
}
