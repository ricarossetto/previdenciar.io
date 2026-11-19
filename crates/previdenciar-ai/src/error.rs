use std::fmt;

#[derive(Debug)]
pub enum AiError {
    Invalid(String),
    MissingKey,
    Vault(String),
    Network(String),
    Provider(String),
    Data(String),
    Protocol(String),
    Cancelled,
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message)
            | Self::Vault(message)
            | Self::Network(message)
            | Self::Provider(message)
            | Self::Data(message)
            | Self::Protocol(message) => f.write_str(message),
            Self::MissingKey => f.write_str("Nenhuma chave de API foi salva para este provedor."),
            Self::Cancelled => f.write_str("Geracao cancelada."),
        }
    }
}

impl std::error::Error for AiError {}

impl From<std::io::Error> for AiError {
    fn from(error: std::io::Error) -> Self {
        let message = match error.kind() {
            std::io::ErrorKind::NotFound => "Arquivo local necessário não encontrado.".to_owned(),
            std::io::ErrorKind::PermissionDenied => {
                "Acesso ao arquivo local foi recusado.".to_owned()
            }
            _ => format!("Falha de leitura ou gravacao local ({:?}).", error.kind()),
        };
        Self::Data(message)
    }
}

pub type Result<T> = std::result::Result<T, AiError>;

