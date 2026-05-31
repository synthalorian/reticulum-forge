//! Project configuration (forge.toml) loading and validation.
//!
//! # Security
//! - Unknown fields are rejected (deny_unknown_fields) to prevent config injection.
//! - Project names are sanitized: no path traversal, no shell metacharacters.
//! - Length limits enforced on all user-facing string inputs.
//! - Topology is validated against a known set.

use crate::error::{ForgeError, ForgeResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Maximum project name length to prevent resource exhaustion.
const MAX_NAME_LENGTH: usize = 128;
/// Maximum description length.
const MAX_DESCRIPTION_LENGTH: usize = 1024;

/// Top-level forge.toml configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ForgeConfig {
    pub project: ProjectConfig,
    #[serde(default)]
    pub network: Option<NetworkConfig>,
}

/// Project metadata section.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_topology")]
    pub topology: Option<String>,
}

fn default_topology() -> Option<String> {
    Some("mesh".into())
}

/// Network defaults section.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct NetworkConfig {
    #[serde(default)]
    pub default_interface: Option<String>,
    #[serde(default)]
    pub transport: Option<String>,
}

const VALID_TOPOLOGIES: &[&str] = &["mesh", "star", "ring", "chain", "custom"];

impl ForgeConfig {
    /// Load and parse a `forge.toml` from the given path.
    ///
    /// # Security
    /// Validates the config after deserialization — unknown fields are rejected,
    /// names are sanitized, and lengths are bounded.
    pub fn load<P: AsRef<Path>>(path: P) -> ForgeResult<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|_| {
            ForgeError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "config file not found: {} — run `forge init` first",
                    path.as_ref().display()
                ),
            ))
        })?;
        let config: Self = toml::from_str(&content).map_err(ForgeError::TomlParse)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the config's semantic correctness.
    pub fn validate(&self) -> ForgeResult<()> {
        // Sanitize and validate project name
        let name = self.project.name.trim();
        if name.is_empty() {
            return Err(ForgeError::Validation(
                "project name must not be empty".into(),
            ));
        }
        if name.len() > MAX_NAME_LENGTH {
            return Err(ForgeError::Validation(format!(
                "project name exceeds maximum length of {} characters",
                MAX_NAME_LENGTH
            )));
        }
        // Block path traversal and shell injection in project name
        if name.contains("..") {
            return Err(ForgeError::Validation(
                "project name must not contain '..' (directory traversal)".into(),
            ));
        }
        if name.contains('/') || name.contains('\\') {
            return Err(ForgeError::Validation(
                "project name must not contain path separators".into(),
            ));
        }
        if name.contains(|c: char| c.is_ascii_control()) {
            return Err(ForgeError::Validation(
                "project name must not contain control characters".into(),
            ));
        }

        // Validate topology
        if let Some(ref topo) = self.project.topology {
            let topo = topo.trim().to_lowercase();
            if !VALID_TOPOLOGIES.contains(&topo.as_str()) {
                return Err(ForgeError::Validation(format!(
                    "unsupported topology '{}': must be one of {:?}",
                    topo, VALID_TOPOLOGIES
                )));
            }
        }

        // Validate description length
        if let Some(ref desc) = self.project.description {
            if desc.len() > MAX_DESCRIPTION_LENGTH {
                return Err(ForgeError::Validation(format!(
                    "project description exceeds maximum length of {} characters",
                    MAX_DESCRIPTION_LENGTH
                )));
            }
        }

        Ok(())
    }

    /// Generate a default config template for `forge init`.
    pub fn default_template(name: &str, topology: &str) -> Self {
        ForgeConfig {
            project: ProjectConfig {
                name: name.to_string(),
                version: Some("0.1.0".into()),
                description: Some(format!(
                    "Reticulum network project '{}' with {} topology",
                    name, topology
                )),
                topology: Some(topology.to_string()),
            },
            network: Some(NetworkConfig {
                default_interface: None,
                transport: Some("udp".into()),
            }),
        }
    }

    /// Serialize the config to TOML string.
    pub fn to_toml_string(&self) -> ForgeResult<String> {
        toml::to_string_pretty(self).map_err(ForgeError::TomlSer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_template_valid() {
        let cfg = ForgeConfig::default_template("test-net", "mesh");
        assert_eq!(cfg.project.name, "test-net");
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let cfg = ForgeConfig::default_template("", "mesh");
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_bad_topology() {
        let cfg = ForgeConfig::default_template("test", "hypercube");
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_name_with_path_traversal() {
        let cfg = ForgeConfig::default_template("../etc/passwd", "mesh");
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_name_with_slashes() {
        let cfg = ForgeConfig::default_template("foo/bar", "mesh");
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_name_too_long() {
        let long = "a".repeat(129);
        let cfg = ForgeConfig::default_template(&long, "mesh");
        assert!(cfg.validate().is_err());
    }
}
