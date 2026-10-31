use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Invalid URL or URI: {0}")]
    InvalidUri(String),

    #[error("Unsupported protocol scheme: {0}")]
    UnsupportedProtocol(String),

    #[error("Base64 decoding error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("JSON serialization or deserialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error("Configuration generation error: {0}")]
    ConfigGenError(String),

    #[error("Invalid configuration field '{field}': {reason}")]
    ValidationError {
        field: String,
        reason: String,
    },

    #[error("General error: {0}")]
    General(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
