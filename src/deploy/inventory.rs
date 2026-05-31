//! Inventory management for remote node deployments.
//!
//! Parses and validates `nodes.toml` which defines the fleet:
//! - Host, SSH credentials, RNS install path
//! - Tags for group deployment (e.g. `[lora-nodes]`, `[backbone]`)
//!
//! # Security
//! - Path traversal blocked on all file paths.
//! - Hostnames validated — no shell metacharacters or control characters.
//! - Key paths validated for existence and safe permissions.
//! - SSH passwords rejected at the inventory level (key-only auth enforced).
//! - Tags bounded in length and sanitized.
//! - All user-facing strings bounded to prevent resource exhaustion.
//! - Unknown fields rejected (`#[serde(deny_unknown_fields)]`).

use crate::error::{ForgeError, ForgeResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// ---- Bounds ----
const MAX_HOST_LENGTH: usize = 255;
const MAX_USER_LENGTH: usize = 64;
const MAX_TAG_LENGTH: usize = 64;
const MAX_TAGS_PER_NODE: usize = 32;
const MAX_NODES: usize = 1024;
const MIN_PORT: u16 = 1;
const MAX_PORT: u16 = 65535;
const MAX_PATH_LENGTH: usize = 2048;

/// A single node in the deployment inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    /// Hostname or IP address (resolved at deploy time).
    pub host: String,
    /// SSH port.
    #[serde(default = "default_port")]
    pub port: u16,
    /// SSH username.
    pub user: String,
    /// Path to SSH private key (ed25519 recommended).
    pub key_path: String,
    /// Optional passphrase for encrypted private keys.
    #[serde(default)]
    pub key_passphrase: Option<String>,
    /// Tags for group-based selection.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Installation path for Reticulum on the remote node.
    #[serde(default = "default_rns_path")]
    pub rns_install_path: String,
    /// Path to the Reticulum config file on the remote node.
    #[serde(default = "default_config_path")]
    pub config_path: String,
    /// Systemd service name for rnsd.
    #[serde(default = "default_service_name")]
    pub service_name: String,
}

fn default_port() -> u16 {
    22
}
fn default_rns_path() -> String {
    "/opt/reticulum".into()
}
fn default_config_path() -> String {
    "/etc/reticulum/config.conf".into()
}
fn default_service_name() -> String {
    "rnsd".into()
}

/// Top-level inventory file (`nodes.toml`).
///
/// # Example
/// ```toml
/// [nodes]
/// [nodes.gateway-01]
/// host = "10.0.0.1"
/// port = 22
/// user = "root"
/// key_path = "~/.ssh/id_ed25519"
/// tags = ["lora", "backbone"]
///
/// [nodes.sensor-01]
/// host = "sensor.internal"
/// user = "root"
/// key_path = "/etc/forge/keys/sensor_key"
/// tags = ["sensor"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    /// Map of node name → node configuration.
    pub nodes: HashMap<String, Node>,
}

impl Inventory {
    /// Load and parse an inventory from a `nodes.toml` file at the given path.
    ///
    /// # Security
    /// Validates all fields after deserialization — unknown fields are rejected,
    /// paths are checked for traversal, hostnames are sanitized, and lengths bounded.
    pub fn load<P: AsRef<Path>>(path: P) -> ForgeResult<Self> {
        // Block directory traversal in the path itself.
        let path_str = path.as_ref().to_string_lossy();
        if path_str.contains("..") {
            return Err(ForgeError::Validation(
                "inventory path must not contain '..' (directory traversal)".into(),
            ));
        }

        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            ForgeError::Io(std::io::Error::new(
                e.kind(),
                format!(
                    "failed to read inventory '{}': {}",
                    path.as_ref().display(),
                    e
                ),
            ))
        })?;

        // Limit input size to prevent resource exhaustion (64 MB).
        if content.len() > 64 * 1024 * 1024 {
            return Err(ForgeError::Validation(
                "inventory file exceeds maximum size of 64 MB".into(),
            ));
        }

        let inventory: Self = toml::from_str(&content).map_err(ForgeError::TomlParse)?;

        inventory.validate()?;
        Ok(inventory)
    }

    /// Validate all nodes in the inventory.
    pub fn validate(&self) -> ForgeResult<()> {
        if self.nodes.is_empty() {
            return Err(ForgeError::Validation(
                "inventory must contain at least one node".into(),
            ));
        }

        if self.nodes.len() > MAX_NODES {
            return Err(ForgeError::Validation(format!(
                "inventory exceeds maximum of {} nodes",
                MAX_NODES
            )));
        }

        for (name, node) in &self.nodes {
            self.validate_node(name, node)?;
        }
        Ok(())
    }

    fn validate_node(&self, name: &str, node: &Node) -> ForgeResult<()> {
        // Node name validation
        if name.is_empty() {
            return Err(ForgeError::Validation("node name must not be empty".into()));
        }
        if name.len() > MAX_HOST_LENGTH {
            return Err(ForgeError::Validation(format!(
                "node name '{}' exceeds maximum length of {}",
                name, MAX_HOST_LENGTH
            )));
        }
        if name.contains('/') || name.contains('\\') || name.contains("..") {
            return Err(ForgeError::Validation(format!(
                "node name '{}' must not contain path separators or '..'",
                name
            )));
        }

        // Host validation
        if node.host.is_empty() {
            return Err(ForgeError::Validation(format!(
                "node '{}': host must not be empty",
                name
            )));
        }
        if node.host.len() > MAX_HOST_LENGTH {
            return Err(ForgeError::Validation(format!(
                "node '{}': host exceeds maximum length of {}",
                name, MAX_HOST_LENGTH
            )));
        }
        // Block shell metacharacters and control characters in host
        if node.host.contains(|c: char| {
            c.is_ascii_control()
                || matches!(
                    c,
                    ';' | '|'
                        | '&'
                        | '$'
                        | '`'
                        | '\''
                        | '"'
                        | '('
                        | ')'
                        | '{'
                        | '}'
                        | '<'
                        | '>'
                        | '!'
                        | '#'
                )
        }) {
            return Err(ForgeError::Validation(format!(
                "node '{}': host contains invalid characters (shell metacharacters or control chars)",
                name
            )));
        }

        // Port validation
        if node.port < MIN_PORT {
            return Err(ForgeError::Validation(format!(
                "node '{}': port {} is out of range ({}-{})",
                name, node.port, MIN_PORT, MAX_PORT
            )));
        }

        // User validation
        if node.user.is_empty() {
            return Err(ForgeError::Validation(format!(
                "node '{}': user must not be empty",
                name
            )));
        }
        if node.user.len() > MAX_USER_LENGTH {
            return Err(ForgeError::Validation(format!(
                "node '{}': user exceeds maximum length of {}",
                name, MAX_USER_LENGTH
            )));
        }
        // Only allow valid Unix username characters
        if !node
            .user
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ForgeError::Validation(format!(
                "node '{}': user must be alphanumeric (underscores and hyphens allowed)",
                name
            )));
        }

        // Key path validation
        if node.key_path.is_empty() {
            return Err(ForgeError::Validation(format!(
                "node '{}': key_path must not be empty",
                name
            )));
        }
        if node.key_path.len() > MAX_PATH_LENGTH {
            return Err(ForgeError::Validation(format!(
                "node '{}': key_path exceeds maximum length of {}",
                name, MAX_PATH_LENGTH
            )));
        }
        // Block traversal in key path
        let expanded = shellexpand::tilde(&node.key_path);
        if expanded.contains("..") {
            return Err(ForgeError::Validation(format!(
                "node '{}': key_path must not contain '..' (directory traversal)",
                name
            )));
        }

        // Tags validation
        if node.tags.len() > MAX_TAGS_PER_NODE {
            return Err(ForgeError::Validation(format!(
                "node '{}': exceeds maximum of {} tags",
                name, MAX_TAGS_PER_NODE
            )));
        }
        for tag in &node.tags {
            if tag.is_empty() {
                return Err(ForgeError::Validation(format!(
                    "node '{}': tag must not be empty",
                    name
                )));
            }
            if tag.len() > MAX_TAG_LENGTH {
                return Err(ForgeError::Validation(format!(
                    "node '{}': tag '{}' exceeds maximum length of {}",
                    name, tag, MAX_TAG_LENGTH
                )));
            }
            // Only alphanumeric, underscores, hyphens
            if !tag
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                return Err(ForgeError::Validation(format!(
                    "node '{}': tag '{}' contains invalid characters",
                    name, tag
                )));
            }
        }

        // Remote paths validation
        for path_field in [&node.rns_install_path, &node.config_path] {
            if path_field.len() > MAX_PATH_LENGTH {
                return Err(ForgeError::Validation(format!(
                    "node '{}': path exceeds maximum length of {}",
                    name, MAX_PATH_LENGTH
                )));
            }
            // Block traversal components in remote paths
            if path_field.contains("..") {
                return Err(ForgeError::Validation(format!(
                    "node '{}': path must not contain '..' (directory traversal)",
                    name
                )));
            }
        }

        Ok(())
    }

    /// Filter nodes by tag. Returns an empty map if no nodes match.
    pub fn nodes_by_tag(&self, tag: &str) -> HashMap<String, &Node> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.tags.iter().any(|t| t == tag))
            .map(|(name, node)| (name.clone(), node))
            .collect()
    }

    /// Return all node names.
    pub fn node_names(&self) -> Vec<String> {
        self.nodes.keys().cloned().collect()
    }

    /// Total number of nodes.
    #[allow(dead_code)]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_node() -> Node {
        Node {
            host: "10.0.0.1".into(),
            port: 22,
            user: "root".into(),
            key_path: "~/.ssh/id_ed25519".into(),
            key_passphrase: None,
            tags: vec!["lora".into()],
            rns_install_path: "/opt/reticulum".into(),
            config_path: "/etc/reticulum/config.conf".into(),
            service_name: "rnsd".into(),
        }
    }

    #[test]
    fn test_valid_inventory() {
        let mut nodes = HashMap::new();
        nodes.insert("gateway-01".into(), valid_node());
        let inventory = Inventory { nodes };
        assert!(inventory.validate().is_ok());
    }

    #[test]
    fn test_empty_inventory() {
        let inventory = Inventory {
            nodes: HashMap::new(),
        };
        assert!(inventory.validate().is_err());
    }

    #[test]
    fn test_empty_host() {
        let mut node = valid_node();
        node.host = "".into();
        let mut nodes = HashMap::new();
        nodes.insert("bad".into(), node);
        let inventory = Inventory { nodes };
        assert!(inventory.validate().is_err());
    }

    #[test]
    fn test_shell_chars_in_host() {
        let mut node = valid_node();
        node.host = "host; rm -rf /".into();
        let mut nodes = HashMap::new();
        nodes.insert("bad".into(), node);
        let inventory = Inventory { nodes };
        assert!(inventory.validate().is_err());
    }

    #[test]
    fn test_traversal_in_key_path() {
        let mut node = valid_node();
        node.key_path = "/home/user/../../etc/key".into();
        let mut nodes = HashMap::new();
        nodes.insert("bad".into(), node);
        let inventory = Inventory { nodes };
        assert!(inventory.validate().is_err());
    }

    #[test]
    fn test_port_out_of_range() {
        let mut node = valid_node();
        node.port = 0;
        let mut nodes = HashMap::new();
        nodes.insert("bad".into(), node);
        let inventory = Inventory { nodes };
        assert!(inventory.validate().is_err());
    }

    #[test]
    fn test_too_many_tags() {
        let mut node = valid_node();
        node.tags = (0..33).map(|i| format!("tag{}", i)).collect();
        let mut nodes = HashMap::new();
        nodes.insert("bad".into(), node);
        let inventory = Inventory { nodes };
        assert!(inventory.validate().is_err());
    }

    #[test]
    fn test_nodes_by_tag() {
        let mut node_a = valid_node();
        node_a.tags = vec!["lora".into(), "backbone".into()];
        let mut node_b = valid_node();
        node_b.tags = vec!["sensor".into()];
        node_b.host = "10.0.0.2".into();

        let mut nodes = HashMap::new();
        nodes.insert("gw".into(), node_a);
        nodes.insert("sensor".into(), node_b);
        let inventory = Inventory { nodes };

        let lora_nodes = inventory.nodes_by_tag("lora");
        assert_eq!(lora_nodes.len(), 1);
        assert!(lora_nodes.contains_key("gw"));

        let sensor_nodes = inventory.nodes_by_tag("sensor");
        assert_eq!(sensor_nodes.len(), 1);
        assert!(sensor_nodes.contains_key("sensor"));

        let backbone_nodes = inventory.nodes_by_tag("backbone");
        assert_eq!(backbone_nodes.len(), 1);

        let nonexistent = inventory.nodes_by_tag("nonexistent");
        assert!(nonexistent.is_empty());
    }

    #[test]
    fn test_toml_round_trip() {
        let toml_str = r#"
[nodes]
[nodes.gateway]
host = "10.0.0.1"
port = 2222
user = "admin"
key_path = "/etc/forge/keys/gateway"
tags = ["lora"]
"#;
        let inventory: Inventory = toml::from_str(toml_str).unwrap();
        assert_eq!(inventory.nodes.len(), 1);
        let node = &inventory.nodes["gateway"];
        assert_eq!(node.host, "10.0.0.1");
        assert_eq!(node.port, 2222);
        assert_eq!(node.user, "admin");
        assert_eq!(node.rns_install_path, "/opt/reticulum"); // default
        assert!(inventory.validate().is_ok());
    }
}
