//! Node status data model and SSH polling for the TUI monitor.
//!
//! Manages per-node state, health polling via SSH, and the shared
//! monitor state that drives the UI render.
//!
//! # Security
//! - Polling reuses the validated deploy inventory (no re-parsing).
//! - SSH connections are short-lived per poll (no persistent sessions).
//! - Connection timeouts prevent hang-on-unreachable.
//! - Failed polls degrade gracefully (node shown as offline).

use crate::deploy::health::HealthStatus;
use crate::deploy::inventory::Inventory;
use std::time::Instant;

/// A single node's display state for the TUI.
#[derive(Debug, Clone)]
pub struct NodeRow {
    /// Node name (key in inventory).
    pub name: String,
    /// SSH host.
    pub host: String,
    /// Current health status.
    pub health: HealthStatus,
    /// Detail text from health check.
    pub health_detail: String,
    /// Human-readable uptime string fetched from the node.
    pub uptime: String,
    /// When this node was last checked.
    pub last_check: Instant,
    /// Whether the detail panel is expanded for this node.
    pub expanded: bool,
    /// RSSI / signal / interface info (future use).
    #[allow(dead_code)]
    pub interface_info: String,
}

/// Application-level status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppStatus {
    /// Normal running mode — keyboard navigates the list.
    Running,
    /// User is typing a filter string — keyboard feeds the filter.
    Filtering,
    /// Quit signal received — break the event loop.
    Quitting,
}

/// Shared monitor state that drives UI rendering.
#[derive(Debug, Clone)]
pub struct MonitorState {
    /// All known node rows.
    pub nodes: Vec<NodeRow>,
    /// Currently selected index in `nodes` (post-filter).
    pub selected_index: usize,
    /// Application status.
    pub status: AppStatus,
    /// Current filter string (matched against node names + host).
    pub filter: String,
    /// Event log entries (most recent first).
    pub log: Vec<LogEntry>,
    /// Timestamp of the last full poll.
    pub last_poll: Instant,
    /// Total nodes by status (summary counters).
    pub summary: SummaryCounts,
}

/// Aggregated node counts for the summary bar.
#[derive(Debug, Clone, Default)]
pub struct SummaryCounts {
    pub total: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub offline: usize,
}

/// A timestamped log entry.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub message: String,
}

impl MonitorState {
    /// Create initial state from an inventory. Does NOT perform any SSH calls.
    pub fn from_inventory(inventory: &Inventory) -> Self {
        let nodes: Vec<NodeRow> = inventory
            .nodes
            .iter()
            .map(|(name, node)| NodeRow {
                name: name.clone(),
                host: node.host.clone(),
                health: HealthStatus::Offline("pending first check".into()),
                health_detail: String::new(),
                uptime: String::new(),
                last_check: Instant::now(),
                expanded: false,
                interface_info: String::new(),
            })
            .collect();

        let total = nodes.len();
        MonitorState {
            nodes,
            selected_index: 0,
            status: AppStatus::Running,
            filter: String::new(),
            log: vec![LogEntry {
                timestamp: format_ts(),
                message: format!("Monitor started. {} nodes in inventory.", total),
            }],
            last_poll: Instant::now(),
            summary: SummaryCounts {
                total,
                ..Default::default()
            },
        }
    }

    /// Returns the subset of nodes matching the current filter.
    pub fn filtered_nodes(&self) -> Vec<&NodeRow> {
        if self.filter.is_empty() {
            self.nodes.iter().collect()
        } else {
            let f = self.filter.to_lowercase();
            self.nodes
                .iter()
                .filter(|n| {
                    n.name.to_lowercase().contains(&f) || n.host.to_lowercase().contains(&f)
                })
                .collect()
        }
    }

    /// Clamp the selected index to valid bounds for the filtered list.
    pub fn clamp_selection(&mut self) {
        let count = self.filtered_nodes().len();
        if count == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= count {
            self.selected_index = count - 1;
        }
    }

    /// Move selection up (toward index 0).
    pub fn select_prev(&mut self) {
        let count = self.filtered_nodes().len();
        if count > 0 {
            if self.selected_index == 0 {
                self.selected_index = count - 1; // wrap
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Move selection down (away from 0).
    pub fn select_next(&mut self) {
        let count = self.filtered_nodes().len();
        if count > 0 {
            if self.selected_index >= count - 1 {
                self.selected_index = 0; // wrap
            } else {
                self.selected_index += 1;
            }
        }
    }

    /// Toggle detail expansion for the currently selected node.
    pub fn toggle_detail(&mut self) {
        let filtered = self.filtered_nodes();
        if let Some(row) = filtered.get(self.selected_index) {
            let global_idx = self.global_index(row.name.as_str());
            if let Some(gi) = global_idx {
                self.nodes[gi].expanded = !self.nodes[gi].expanded;
            }
        }
    }

    /// Find global index of a node by name.
    fn global_index(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.name == name)
    }

    /// Rebuild summary counts from current node statuses.
    pub fn recompute_summary(&mut self) {
        let s = SummaryCounts {
            total: self.nodes.len(),
            healthy: self.nodes.iter().filter(|n| n.health.is_healthy()).count(),
            degraded: self.nodes.iter().filter(|n| n.health.is_degraded()).count(),
            offline: self.nodes.iter().filter(|n| n.health.is_offline()).count(),
        };
        self.summary = s;
    }

    /// Add a log entry with auto-timestamp.
    pub fn add_log(&mut self, message: impl Into<String>) {
        self.log.insert(
            0,
            LogEntry {
                timestamp: format_ts(),
                message: message.into(),
            },
        );
        // Keep last 100 entries
        self.log.truncate(100);
    }
}

fn format_ts() -> String {
    let now = chrono::Local::now();
    now.format("%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy::inventory::Node;

    fn test_inventory() -> Inventory {
        let mut nodes = std::collections::HashMap::new();
        nodes.insert(
            "gw".into(),
            Node {
                host: "10.0.0.1".into(),
                port: 22,
                user: "root".into(),
                key_path: "~/.ssh/id_ed25519".into(),
                key_passphrase: None,
                tags: vec!["test".into()],
                rns_install_path: "/opt/reticulum".into(),
                config_path: "/etc/reticulum/config.conf".into(),
                service_name: "rnsd".into(),
            },
        );
        nodes.insert(
            "sensor".into(),
            Node {
                host: "10.0.0.2".into(),
                port: 22,
                user: "root".into(),
                key_path: "~/.ssh/id_ed25519".into(),
                key_passphrase: None,
                tags: vec!["sensor".into()],
                rns_install_path: "/opt/reticulum".into(),
                config_path: "/etc/reticulum/config.conf".into(),
                service_name: "rnsd".into(),
            },
        );
        Inventory { nodes }
    }

    #[test]
    fn test_initial_state_creates_rows() {
        let inv = test_inventory();
        let state = MonitorState::from_inventory(&inv);
        assert_eq!(state.nodes.len(), 2);
        let names: Vec<&str> = state.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(names.contains(&"gw"));
        assert!(names.contains(&"sensor"));
    }

    #[test]
    fn test_filtered_nodes() {
        let inv = test_inventory();
        let mut state = MonitorState::from_inventory(&inv);
        assert_eq!(state.filtered_nodes().len(), 2);

        state.filter = "gw".into();
        assert_eq!(state.filtered_nodes().len(), 1);
        assert_eq!(state.filtered_nodes()[0].name, "gw");
    }

    #[test]
    fn test_selection_wrapping() {
        let inv = test_inventory();
        let mut state = MonitorState::from_inventory(&inv);

        assert_eq!(state.selected_index, 0);
        state.select_prev();
        assert_eq!(state.selected_index, 1); // wraps to last
        state.select_next();
        assert_eq!(state.selected_index, 0); // wraps to first
    }

    #[test]
    fn test_summary_counts() {
        let inv = test_inventory();
        let mut state = MonitorState::from_inventory(&inv);

        // All start as Offline (pending first check)
        state.recompute_summary();
        assert_eq!(state.summary.total, 2);
        assert_eq!(state.summary.offline, 2);

        // Mark one healthy
        state.nodes[0].health = HealthStatus::Healthy;
        state.recompute_summary();
        assert_eq!(state.summary.healthy, 1);
        assert_eq!(state.summary.offline, 1);
    }

    #[test]
    fn test_toggle_detail() {
        let inv = test_inventory();
        let mut state = MonitorState::from_inventory(&inv);

        assert!(!state.nodes[0].expanded);
        state.toggle_detail();
        assert!(state.nodes[0].expanded);
        state.toggle_detail();
        assert!(!state.nodes[0].expanded);
    }

    #[test]
    fn test_add_log() {
        let inv = test_inventory();
        let mut state = MonitorState::from_inventory(&inv);

        state.add_log("test message");
        assert_eq!(state.log.len(), 2); // initial + new
        assert!(state.log[0].message.contains("test message"));
    }
}
