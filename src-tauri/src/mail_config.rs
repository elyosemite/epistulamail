use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Imap,
    Pop3,
    Graph,
}

impl Protocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Protocol::Imap => "imap",
            Protocol::Pop3 => "pop3",
            Protocol::Graph => "graph",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "imap" => Some(Protocol::Imap),
            "pop3" => Some(Protocol::Pop3),
            "graph" => Some(Protocol::Graph),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    Password,
    Encrypted,
    Gssapi,
    Ntlm,
    OAuth2,
}

impl AuthMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthMethod::Password => "password",
            AuthMethod::Encrypted => "encrypted",
            AuthMethod::Gssapi => "gssapi",
            AuthMethod::Ntlm => "ntlm",
            AuthMethod::OAuth2 => "oauth2",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "password" => Some(AuthMethod::Password),
            "encrypted" => Some(AuthMethod::Encrypted),
            "gssapi" => Some(AuthMethod::Gssapi),
            "ntlm" => Some(AuthMethod::Ntlm),
            "oauth2" => Some(AuthMethod::OAuth2),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "tls" => Some(Security::Tls),
            "starttls" => Some(Security::StartTls),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
    pub security: Security,
    pub auth: AuthMethod,
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailConfig {
    pub protocol: Protocol,
    pub incoming: Endpoint,
    pub outgoing: Endpoint,
    pub outgoing_shared: bool,
}

impl MailConfig {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.protocol != Protocol::Imap {
            return Err(AppError::ProviderUnavailable);
        }
        validate_endpoint(&self.incoming)?;
        validate_endpoint(&self.outgoing)?;
        if !self.incoming.username.contains('@') {
            return Err(AppError::InvalidInput);
        }
        if self.outgoing_shared && self.incoming.username != self.outgoing.username {
            return Err(AppError::InvalidInput);
        }
        Ok(())
    }

    pub fn account_email(&self) -> &str {
        &self.incoming.username
    }
}

fn validate_endpoint(endpoint: &Endpoint) -> Result<(), AppError> {
    let host = endpoint.host.trim();
    if host.is_empty()
        || host != endpoint.host
        || host.len() > 253
        || host.chars().any(|c| c.is_control() || c.is_whitespace())
        || host.contains(['/', '\\', '?', '#', '@'])
    {
        return Err(AppError::InvalidInput);
    }
    if endpoint.port == 0 {
        return Err(AppError::InvalidInput);
    }
    let username = endpoint.username.trim();
    if username.is_empty() || username != endpoint.username || username.chars().any(char::is_control) || username.len() > 320 {
        return Err(AppError::InvalidInput);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(host: &str, port: u16, username: &str) -> Endpoint {
        Endpoint {
            host: host.into(),
            port,
            security: Security::Tls,
            auth: AuthMethod::Password,
            username: username.into(),
        }
    }

    #[test]
    fn parses_stored_values() {
        assert_eq!(Protocol::parse("imap"), Some(Protocol::Imap));
        assert_eq!(AuthMethod::parse("encrypted"), Some(AuthMethod::Encrypted));
        assert_eq!(AuthMethod::parse("ntlm"), Some(AuthMethod::Ntlm));
        assert_eq!(Security::parse("starttls"), Some(Security::StartTls));
        assert_eq!(Protocol::parse("smtp"), None);
        assert_eq!(AuthMethod::parse("none"), None);
        assert_eq!(Security::parse("none"), None);
    }

    #[test]
    fn accepts_a_manual_imap_config() {
        let config = MailConfig {
            protocol: Protocol::Imap,
            incoming: endpoint("mail.exemplo.com", 993, "ana@exemplo.com"),
            outgoing: endpoint("mail.exemplo.com", 465, "ana@exemplo.com"),
            outgoing_shared: true,
        };
        assert_eq!(config.validate(), Ok(()));
        assert_eq!(config.account_email(), "ana@exemplo.com");
    }

    #[test]
    fn refuses_host_with_scheme_or_path() {
        let mut config = MailConfig {
            protocol: Protocol::Imap,
            incoming: endpoint("https://mail.exemplo.com", 993, "ana@exemplo.com"),
            outgoing: endpoint("mail.exemplo.com", 465, "ana@exemplo.com"),
            outgoing_shared: true,
        };
        assert_eq!(config.validate(), Err(AppError::InvalidInput));
        config.incoming.host = "mail.exemplo.com/path".into();
        assert_eq!(config.validate(), Err(AppError::InvalidInput));
    }

    #[test]
    fn refuses_empty_port_and_username() {
        let mut config = MailConfig {
            protocol: Protocol::Imap,
            incoming: endpoint("mail.exemplo.com", 0, "ana@exemplo.com"),
            outgoing: endpoint("mail.exemplo.com", 465, "ana@exemplo.com"),
            outgoing_shared: true,
        };
        assert_eq!(config.validate(), Err(AppError::InvalidInput));
        config.incoming.port = 993;
        config.incoming.username = "  ".into();
        assert_eq!(config.validate(), Err(AppError::InvalidInput));
    }
}
