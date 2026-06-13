//! Error types for Reticulum Forge.

use thiserror::Error;

/// Unified error type for the Forge CLI toolkit.
#[derive(Error, Debug)]
pub enum ForgeError {
    /// Filesystem I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Tera template rendering error.
    #[error("Template error: {0}")]
    Template(#[from] tera::Error),

    /// Template engine initialization error.
    #[error("Template init error: {0}")]
    TeraInit(String),

    /// TOML parse error.
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// TOML serialization error.
    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    /// JSON serialization error.
    #[error("JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    /// YAML serialization error.
    #[error("YAML error: {0}")]
    SerdeYaml(#[from] serde_yaml::Error),

    /// Validation error.
    #[error("Validation error: {0}")]
    Validation(String),

    /// CLI usage error.
    #[error("{0}")]
    Cli(String),

    /// SSH connection or command error.
    #[error("SSH error: {0}")]
    Ssh(String),

    /// Deploy orchestration error.
    #[error("Deploy error: {0}")]
    Deploy(String),

    /// Rollback error.
    #[error("Rollback error: {0}")]
    Rollback(String),
}

/// Convenience alias for results using `ForgeError`.
pub type ForgeResult<T> = Result<T, ForgeError>;
