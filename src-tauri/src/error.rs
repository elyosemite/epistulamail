use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Erro tipado que chega ao frontend como `{ kind, message }`.
/// A mensagem é sempre texto amigável; detalhes técnicos só vão para o log.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AppError {
    #[error("Você cancelou o login no navegador.")]
    AuthCancelled,
    #[error("O login demorou demais e foi interrompido.")]
    AuthTimeout,
    #[error("O provedor não autorizou o acesso. Tente de novo.")]
    AuthFailed,
    #[error("Sua sessão expirou. Conecte a conta de novo.")]
    TokenExpired,
    #[error("Não foi possível falar com o servidor de e-mail. Verifique sua conexão e tente de novo.")]
    ImapConnection,
    #[error("O servidor de e-mail recusou o acesso desta conta. Conecte de novo.")]
    ImapAuth,
    #[error("Não foi possível acessar o cofre de credenciais do sistema.")]
    Keyring,
    #[error("A pasta mudou desde a última leitura. Recarregando a lista.")]
    CursorExpired,
    #[error("Esta mensagem não está mais nesta pasta.")]
    MessageNotFound,
    #[error("Esta mensagem é grande demais para abrir no Epistula (limite de 25 MB).")]
    MessageTooLarge,
    #[error("Referência inválida. Recarregue a lista.")]
    InvalidId,
    #[error("Este link não pode ser aberto.")]
    UnsupportedLink,
    #[error("O Epistula não está configurado para este provedor.")]
    Config,
    #[error("Não foi possível acessar os dados salvos neste computador.")]
    Storage,
    #[error("Esta conta não está mais no Epistula.")]
    AccountNotFound,
    #[error("Este provedor ainda não está disponível no Epistula.")]
    ProviderUnavailable,
    #[error("Você saiu desta conta. Entre de novo para atualizar as mensagens.")]
    SignedOut,
    #[error("Valor inválido.")]
    InvalidInput,
    #[error("Já existe uma pasta com esse nome.")]
    FolderExists,
    #[error("Esta conta não tem essa pasta.")]
    FolderNotFound,
    #[error("Esta mensagem está sendo movida. Tente de novo em instantes.")]
    MessageMoving,
    #[error("Não foi possível falar com o servidor de envio. Verifique sua conexão e tente de novo.")]
    SmtpConnection,
    #[error("O servidor de e-mail recusou o envio. Confira os destinatários e tente de novo.")]
    SendRejected,
    #[error("Adicione pelo menos um destinatário.")]
    NoRecipients,
    #[error("Há um destinatário com endereço inválido.")]
    InvalidRecipient,
    #[error("Os anexos passam do limite de 25 MB.")]
    AttachmentsTooLarge,
    #[error("O Epistula está bloqueado. Digite a senha para continuar.")]
    Locked,
    #[error("Senha incorreta.")]
    WrongPassword,
    #[error("Aguarde um pouco antes de tentar de novo.")]
    UnlockWait,
    #[error("Use uma senha com pelo menos 8 caracteres.")]
    WeakPassword,
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::AuthCancelled => "AuthCancelled",
            AppError::AuthTimeout => "AuthTimeout",
            AppError::AuthFailed => "AuthFailed",
            AppError::TokenExpired => "TokenExpired",
            AppError::ImapConnection => "ImapConnection",
            AppError::ImapAuth => "ImapAuth",
            AppError::Keyring => "Keyring",
            AppError::CursorExpired => "CursorExpired",
            AppError::MessageNotFound => "MessageNotFound",
            AppError::MessageTooLarge => "MessageTooLarge",
            AppError::InvalidId => "InvalidId",
            AppError::UnsupportedLink => "UnsupportedLink",
            AppError::Config => "Config",
            AppError::Storage => "Storage",
            AppError::AccountNotFound => "AccountNotFound",
            AppError::ProviderUnavailable => "ProviderUnavailable",
            AppError::SignedOut => "SignedOut",
            AppError::InvalidInput => "InvalidInput",
            AppError::FolderExists => "FolderExists",
            AppError::FolderNotFound => "FolderNotFound",
            AppError::MessageMoving => "MessageMoving",
            AppError::SmtpConnection => "SmtpConnection",
            AppError::SendRejected => "SendRejected",
            AppError::NoRecipients => "NoRecipients",
            AppError::InvalidRecipient => "InvalidRecipient",
            AppError::AttachmentsTooLarge => "AttachmentsTooLarge",
            AppError::Locked => "Locked",
            AppError::WrongPassword => "WrongPassword",
            AppError::UnlockWait => "UnlockWait",
            AppError::WeakPassword => "WeakPassword",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_kind_and_message() {
        let json = serde_json::to_value(AppError::AuthCancelled).unwrap();
        assert_eq!(json["kind"], "AuthCancelled");
        assert_eq!(json["message"], "Você cancelou o login no navegador.");
    }
}
