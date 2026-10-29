//! Catálogo de provedores de e-mail (US3, CA4). Gmail e Outlook são conectáveis; os demais já
//! têm servidores e tipo de autenticação definidos para as próximas stories.

use serde::Serialize;

use crate::error::AppError;
use crate::provider::Provider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthKind {
    OAuth2(Provider),
    /// Senha de app (IMAP/SMTP com usuário e senha).
    Password,
}

impl AuthKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthKind::OAuth2(_) => "oauth2",
            AuthKind::Password => "password",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Security {
    Tls,
    StartTls,
}

impl Security {
    pub fn as_str(self) -> &'static str {
        match self {
            Security::Tls => "tls",
            Security::StartTls => "starttls",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Server {
    pub host: &'static str,
    pub port: u16,
    pub security: Security,
}

#[derive(Debug, Clone, Copy)]
pub struct CatalogEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub available: bool,
    pub auth: AuthKind,
    /// `None` para "Outro": o usuário informa os servidores.
    pub imap: Option<Server>,
    pub smtp: Option<Server>,
}

const fn tls(host: &'static str, port: u16) -> Option<Server> {
    Some(Server { host, port, security: Security::Tls })
}

const fn starttls(host: &'static str, port: u16) -> Option<Server> {
    Some(Server { host, port, security: Security::StartTls })
}

pub const CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        id: "gmail",
        name: "Gmail",
        description: "Entrar com o Google",
        available: true,
        auth: AuthKind::OAuth2(Provider::Google),
        imap: tls("imap.gmail.com", 993),
        smtp: tls("smtp.gmail.com", 465),
    },
    CatalogEntry {
        id: "outlook",
        name: "Outlook",
        description: "Outlook.com, Hotmail e Microsoft 365",
        available: true,
        auth: AuthKind::OAuth2(Provider::Microsoft),
        imap: tls("outlook.office365.com", 993),
        smtp: starttls("smtp.office365.com", 587),
    },
    CatalogEntry {
        id: "yahoo",
        name: "Yahoo",
        description: "Yahoo Mail",
        available: false,
        auth: AuthKind::Password,
        imap: tls("imap.mail.yahoo.com", 993),
        smtp: tls("smtp.mail.yahoo.com", 465),
    },
    CatalogEntry {
        id: "icloud",
        name: "iCloud Mail",
        description: "@icloud.com, @me.com, @mac.com",
        available: false,
        auth: AuthKind::Password,
        imap: tls("imap.mail.me.com", 993),
        smtp: starttls("smtp.mail.me.com", 587),
    },
    CatalogEntry {
        id: "zoho",
        name: "Zoho Mail",
        description: "Zoho Mail pessoal e empresarial",
        available: false,
        auth: AuthKind::Password,
        imap: tls("imap.zoho.com", 993),
        smtp: tls("smtp.zoho.com", 465),
    },
    CatalogEntry {
        id: "gmx",
        name: "GMX",
        description: "GMX Mail",
        available: false,
        auth: AuthKind::Password,
        imap: tls("imap.gmx.com", 993),
        smtp: starttls("mail.gmx.com", 587),
    },
    CatalogEntry {
        id: "aol",
        name: "AOL Mail",
        description: "AOL Mail",
        available: false,
        auth: AuthKind::Password,
        imap: tls("imap.aol.com", 993),
        smtp: tls("smtp.aol.com", 465),
    },
    CatalogEntry {
        id: "custom",
        name: "Outro",
        description: "Domínio próprio (IMAP e SMTP)",
        available: false,
        auth: AuthKind::Password,
        imap: None,
        smtp: None,
    },
];

pub fn find(id: &str) -> Option<&'static CatalogEntry> {
    CATALOG.iter().find(|p| p.id == id)
}

/// Provedor que o usuário pode conectar agora; os demais devolvem `ProviderUnavailable`.
pub fn connectable(id: &str) -> Result<&'static CatalogEntry, AppError> {
    find(id).filter(|p| p.available).ok_or(AppError::ProviderUnavailable)
}

/// Provedor do catálogo para uma conta salva pela US1/US2 (que só conhecia o provedor OAuth).
pub fn for_oauth(provider: Provider) -> &'static str {
    match provider {
        Provider::Google => "gmail",
        Provider::Microsoft => "outlook",
    }
}

#[derive(Debug, Serialize)]
pub struct ProviderView {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub available: bool,
    pub icon: &'static str,
}

pub fn views() -> Vec<ProviderView> {
    CATALOG
        .iter()
        .map(|p| ProviderView {
            id: p.id,
            name: p.name,
            description: p.description,
            available: p.available,
            icon: if p.id == "custom" { "globe" } else { "mail" },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gmail_and_outlook_are_connectable_for_now() {
        assert!(connectable("gmail").is_ok());
        assert!(connectable("outlook").is_ok());
        for id in ["yahoo", "icloud", "zoho", "gmx", "aol", "custom", "inexistente"] {
            assert_eq!(connectable(id).err(), Some(AppError::ProviderUnavailable), "{id}");
        }
    }

    #[test]
    fn catalog_covers_the_story_providers() {
        let ids: Vec<_> = CATALOG.iter().map(|p| p.id).collect();
        assert_eq!(ids, ["gmail", "outlook", "yahoo", "icloud", "zoho", "gmx", "aol", "custom"]);
        assert!(CATALOG.iter().filter(|p| p.id != "custom").all(|p| p.imap.is_some() && p.smtp.is_some()));
    }
}
