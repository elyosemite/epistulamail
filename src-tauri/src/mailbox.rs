//! Pastas: classificação pelos atributos SPECIAL-USE (RFC 6154), nunca pelo nome, que muda com
//! o idioma da conta; e decodificação de nomes em UTF-7 modificado (RFC 3501 §5.1.3).

use async_imap::imap_proto::types::NameAttribute;
use base64::alphabet;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use base64::Engine;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Inbox,
    Starred,
    Sent,
    Drafts,
    All,
    Spam,
    Trash,
    Custom,
}

impl Role {
    pub fn label(self) -> Option<&'static str> {
        match self {
            Role::Inbox => Some("Caixa de entrada"),
            Role::Starred => Some("Com estrela"),
            Role::Sent => Some("Enviados"),
            Role::Drafts => Some("Rascunhos"),
            Role::All => Some("Todos os e-mails"),
            Role::Spam => Some("Spam"),
            Role::Trash => Some("Lixeira"),
            Role::Custom => None,
        }
    }

    pub fn is_system(self) -> bool {
        self != Role::Custom
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Folder {
    pub id: String,
    pub role: Role,
    pub name: String,
    pub unread: Option<u32>,
    pub total: Option<u32>,
    #[serde(skip)]
    pub mailbox: String,
}

/// `None` para pastas que não aparecem: não selecionáveis e a "Importante" do Gmail.
pub fn classify(name: &str, attributes: &[NameAttribute<'_>]) -> Option<Role> {
    if attributes.iter().any(|a| matches!(a, NameAttribute::NoSelect)) {
        return None;
    }
    if name.eq_ignore_ascii_case("INBOX") {
        return Some(Role::Inbox);
    }
    for attribute in attributes {
        let role = match attribute {
            NameAttribute::Flagged => Role::Starred,
            NameAttribute::Sent => Role::Sent,
            NameAttribute::Drafts => Role::Drafts,
            NameAttribute::All | NameAttribute::Archive => Role::All,
            NameAttribute::Junk => Role::Spam,
            NameAttribute::Trash => Role::Trash,
            NameAttribute::Extension(ext) if ext.eq_ignore_ascii_case("\\Important") => return None,
            _ => continue,
        };
        return Some(role);
    }
    Some(Role::Custom)
}

const MODIFIED_BASE64: GeneralPurpose = GeneralPurpose::new(
    &alphabet::IMAP_MUTF7,
    GeneralPurposeConfig::new()
        .with_encode_padding(false)
        .with_decode_padding_mode(DecodePaddingMode::Indifferent)
        .with_decode_allow_trailing_bits(true),
);

/// Decodifica UTF-7 modificado. Sequências inválidas ficam como estão, para nunca perder o nome.
pub fn decode_mailbox_name(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('-') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let encoded = &after[..end];
        if encoded.is_empty() {
            out.push('&');
        } else {
            match MODIFIED_BASE64.decode(encoded) {
                Ok(bytes) if bytes.len() % 2 == 0 => {
                    let units: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
                    out.push_str(&String::from_utf16_lossy(&units));
                }
                _ => out.push_str(&rest[start..start + 1 + end + 1]),
            }
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Pastas do sistema na ordem da US2, depois marcadores em ordem alfabética (sem diferenciar caixa).
pub fn sort_folders(folders: &mut [Folder]) {
    folders.sort_by(|a, b| a.role.cmp(&b.role).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn classifies_by_special_use() {
        assert_eq!(classify("INBOX", &[]), Some(Role::Inbox));
        assert_eq!(classify("[Gmail]/Enviados", &[NameAttribute::Sent]), Some(Role::Sent));
        assert_eq!(classify("[Gmail]/Sent Mail", &[NameAttribute::Sent]), Some(Role::Sent));
        assert_eq!(classify("[Gmail]/Com estrela", &[NameAttribute::Flagged]), Some(Role::Starred));
        assert_eq!(classify("[Gmail]/Todos os e-mails", &[NameAttribute::All]), Some(Role::All));
        assert_eq!(classify("[Gmail]/Spam", &[NameAttribute::Junk]), Some(Role::Spam));
        assert_eq!(classify("[Gmail]/Lixeira", &[NameAttribute::Trash]), Some(Role::Trash));
        assert_eq!(classify("[Gmail]/Rascunhos", &[NameAttribute::Drafts]), Some(Role::Drafts));
        assert_eq!(classify("[Gmail]", &[NameAttribute::NoSelect]), None);
        assert_eq!(classify("[Gmail]/Importante", &[NameAttribute::Extension(Cow::Borrowed("\\Important"))]), None);
        assert_eq!(classify("Projetos", &[NameAttribute::Extension(Cow::Borrowed("\\HasNoChildren"))]), Some(Role::Custom));
    }

    #[test]
    fn decodes_modified_utf7() {
        assert_eq!(decode_mailbox_name("INBOX"), "INBOX");
        assert_eq!(decode_mailbox_name("[Gmail]/Lixeira"), "[Gmail]/Lixeira");
        assert_eq!(decode_mailbox_name("Reuni&AOM-o"), "Reunião");
        assert_eq!(decode_mailbox_name("&ZeVnLIqe-"), "日本語");
        assert_eq!(decode_mailbox_name("Tom &- Jerry"), "Tom & Jerry");
        assert_eq!(decode_mailbox_name("quebrado&AOM"), "quebrado&AOM");
        assert_eq!(decode_mailbox_name("inv&@@@-alido"), "inv&@@@-alido");
    }

    #[test]
    fn sorts_system_first_then_labels() {
        let f = |role, name: &str| Folder { id: String::new(), role, name: name.into(), unread: None, total: None, mailbox: String::new() };
        let mut folders = vec![f(Role::Custom, "zeta"), f(Role::Trash, "Lixeira"), f(Role::Custom, "Alfa"), f(Role::Inbox, "Caixa de entrada")];
        sort_folders(&mut folders);
        let names: Vec<_> = folders.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["Caixa de entrada", "Lixeira", "Alfa", "zeta"]);
    }
}
