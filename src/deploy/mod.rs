//! Deploy orchestration — rolling deployment, parallel mode, dry-run, rollback.
//!
//! Ties together inventory management, SSH connectivity, health checks,
//! provisioning, and rollback into a single deploy workflow.
//!
//! # Security
//! - Dry-run mode never touches remote machines (read-only).
//! - Rollback snapshots are created BEFORE any changes.
//! - Concurrency is bounded by user-specified limit.
//! - Failed health checks after deploy trigger automatic rollback.
//! - All sub-modules enforce their own security constraints.

pub mod health;
pub mod inventory;
pub mod provision;
pub mod rollback;
pub mod ssh;

use crate::deploy::health::{check_node_health_detailed, HealthCheckResult, HealthStatus};
use crate::deploy::inventory::Inventory;
use crate::deploy::provision::provision_node;
use crate::deploy::rollback::{create_snapshot, rollback, RollbackSnapshot};
use crate::deploy::ssh::{SshClient, SshConfig};
use crate::error::{ForgeError, ForgeResult};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Configuration for a deploy run.
#[derive(Debug, Clone)]
pub struct DeployConfig {
    /// Path to the inventory file (nodes.toml).
    pub inventory_path: String,
    /// If true, only print planned actions — no remote changes.
    pub dry_run: bool,
    /// Maximum number of nodes to deploy in parallel.
    pub concurrency: usize,
    /// If true, run full provisioning before deploying config.
    pub provision: bool,
    /// Optional tag filter — only deploy nodes with this tag.
    pub tag_filter: Option<String>,
    /// Config file content to deploy (from a generated or template config).
    pub config_content: Option<String>,
    /// SSH connection timeout in seconds.
    pub timeout_secs: u64,
}

impl Default for DeployConfig {
    fn default() -> Self {
        DeployConfig {
            inventory_path: "nodes.toml".into(),
            dry_run: false,
            concurrency: 1,
            provision: false,
            tag_filter: None,
            config_content: None,
            timeout_secs: 30,
        }
    }
}

/// Status of a single node after deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum DeployNodeStatus {
    Success,
    RolledBack,
    Failed(String),
    Skipped(String),
}

impl DeployNodeStatus {
    pub fn label(&self) -> &str {
        match self {
            DeployNodeStatus::Success => "success",
            DeployNodeStatus::RolledBack => "rolled back",
            DeployNodeStatus::Failed(_) => "failed",
            DeployNodeStatus::Skipped(_) => "skipped",
        }
    }
}

/// Result of deploying to a single node.
#[derive(Debug, Clone, Serialize)]
pub struct DeployNodeResult {
    pub node_name: String,
    pub status: DeployNodeStatus,
    pub details: String,
    pub health_before: Option<HealthCheckResult>,
    pub health_after: Option<HealthCheckResult>,
}

/// Summary of the entire deploy run.
#[derive(Debug, Clone, Serialize)]
pub struct DeploySummary {
    pub total: usize,
    pub success: usize,
    pub rolled_back: usize,
    pub failed: usize,
    pub skipped: usize,
}

/// Comprehensive deploy report.
#[derive(Debug, Clone, Serialize)]
pub struct DeployReport {
    pub node_results: Vec<DeployNodeResult>,
    pub summary: DeploySummary,
}

impl DeployReport {
    /// Format as a human-readable summary table.
    pub fn to_table(&self) -> String {
        let mut output = String::new();
        output.push_str("Deploy Summary:\n");
        output.push_str(&format!(
            "  {} success, {} rolled back, {} failed, {} skipped ({} total)\n",
            self.summary.success,
            self.summary.rolled_back,
            self.summary.failed,
            self.summary.skipped,
            self.summary.total
        ));
        output.push('\n');
        output.push_str("  Nodes:\n");
        for result in &self.node_results {
            let icon = match result.status {
                DeployNodeStatus::Success => "✓",
                DeployNodeStatus::RolledBack => "↩",
                DeployNodeStatus::Failed(_) => "✗",
                DeployNodeStatus::Skipped(_) => "–",
            };
            output.push_str(&format!(
                "    {} {} — {}\n",
                icon,
                result.node_name,
                result.status.label()
            ));
            if !result.details.is_empty() {
                output.push_str(&format!("      {}\n", result.details));
            }
        }
        output
    }

    /// Format as a JSON string.
    pub fn to_json(&self) -> ForgeResult<String> {
        serde_json::to_string_pretty(self).map_err(ForgeError::SerdeJson)
    }
}

/// Orchestrates the full deployment workflow.
pub struct DeployOrchestrator {
    config: DeployConfig,
    inventory: Inventory,
}

impl DeployOrchestrator {
    /// Load inventory and create a new orchestrator.
    pub fn new(config: DeployConfig) -> ForgeResult<Self> {
        let inventory = Inventory::load(&config.inventory_path)?;

        // Apply tag filter
        if let Some(ref tag) = config.tag_filter {
            let filtered = inventory.nodes_by_tag(tag);
            if filtered.is_empty() {
                return Err(ForgeError::Validation(format!(
                    "no nodes found with tag '{}'",
                    tag
                )));
            }
        }

        Ok(DeployOrchestrator { config, inventory })
    }

    /// Run the full deployment.
    pub async fn deploy(&self) -> ForgeResult<DeployReport> {
        let node_names = self.target_nodes();
        let total = node_names.len();

        if self.config.dry_run {
            return Ok(self.dry_run_report(&node_names));
        }

        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));
        let mut handles = Vec::new();

        for node_name in node_names {
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| ForgeError::Deploy("semaphore acquisition failed".into()))?;

            let node = match self.inventory.nodes.get(&node_name) {
                Some(n) => n.clone(),
                None => {
                    return Err(ForgeError::Deploy(format!(
                        "node '{}' not found in inventory (inventory may be corrupted)",
                        node_name
                    )));
                }
            };
            let config = DeployConfig {
                provision: self.config.provision,
                config_content: self.config.config_content.clone(),
                timeout_secs: self.config.timeout_secs,
                ..Default::default()
            };

            handles.push(tokio::spawn(async move {
                let _permit = permit; // held until closure drops
                deploy_single_node(&node_name, &node, &config).await
            }));
        }

        let mut node_results: Vec<DeployNodeResult> = Vec::with_capacity(total);
        for handle in handles {
            match handle.await {
                Ok(result) => node_results.push(result),
                Err(e) => {
                    node_results.push(DeployNodeResult {
                        node_name: "unknown".into(),
                        status: DeployNodeStatus::Failed(format!("task panicked: {}", e)),
                        health_before: None,
                        health_after: None,
                        details: String::new(),
                    });
                }
            }
        }

        let summary = Self::compute_summary(&node_results);
        Ok(DeployReport {
            node_results,
            summary,
        })
    }

    /// Return the list of target node names (filtered by tag if applicable).
    fn target_nodes(&self) -> Vec<String> {
        if let Some(ref tag) = self.config.tag_filter {
            self.inventory.nodes_by_tag(tag).into_keys().collect()
        } else {
            self.inventory.node_names()
        }
    }

    /// Build a report for dry-run mode (no remote actions).
    fn dry_run_report(&self, node_names: &[String]) -> DeployReport {
        let mut node_results: Vec<DeployNodeResult> = Vec::new();
        for name in node_names {
            let Some(node) = self.inventory.nodes.get(name) else {
                continue;
            };
            let mut details = format!(
                "Would deploy to {}@{}:{} (config: {}, service: {})",
                node.user, node.host, node.port, node.config_path, node.service_name
            );
            if self.config.provision {
                details.push_str(" [with provisioning]");
            }
            if let Some(ref tag) = self.config.tag_filter {
                details.push_str(&format!(" [tag filter: {}]", tag));
            }

            node_results.push(DeployNodeResult {
                node_name: name.clone(),
                status: DeployNodeStatus::Skipped("dry-run".into()),
                details,
                health_before: None,
                health_after: None,
            });
        }

        DeployReport {
            summary: DeploySummary {
                total: node_names.len(),
                success: 0,
                rolled_back: 0,
                failed: 0,
                skipped: node_names.len(),
            },
            node_results,
        }
    }

    fn compute_summary(results: &[DeployNodeResult]) -> DeploySummary {
        let mut summary = DeploySummary {
            total: results.len(),
            success: 0,
            rolled_back: 0,
            failed: 0,
            skipped: 0,
        };
        for r in results {
            match r.status {
                DeployNodeStatus::Success => summary.success += 1,
                DeployNodeStatus::RolledBack => summary.rolled_back += 1,
                DeployNodeStatus::Failed(_) => summary.failed += 1,
                DeployNodeStatus::Skipped(_) => summary.skipped += 1,
            }
        }
        summary
    }
}

/// Deploy to a single node: snapshot → provision? → transfer → restart → health check → rollback on failure.
async fn deploy_single_node(
    node_name: &str,
    node: &inventory::Node,
    config: &DeployConfig,
) -> DeployNodeResult {
    let ssh_config = SshConfig {
        host: node.host.clone(),
        port: node.port,
        user: node.user.clone(),
        key_path: node.key_path.clone(),
        key_passphrase: node.key_passphrase.clone(),
        timeout_secs: config.timeout_secs,
        verify_host_key: false,
    };

    // 1. Health check before deploy
    let health_before = check_node_health_detailed(
        node_name,
        &ssh_config,
        &node.service_name,
        &node.config_path,
    )
    .await;

    // If node is offline and we're not provisioning, warn but continue
    if matches!(health_before.status, HealthStatus::Offline(_)) && !config.provision {
        return DeployNodeResult {
            node_name: node_name.to_string(),
            status: DeployNodeStatus::Skipped(
                "node offline (use --provision to attempt setup)".into(),
            ),
            health_before: Some(health_before),
            health_after: None,
            details: "node is unreachable — skipping deploy".into(),
        };
    }

    // 2. Create rollback snapshot (capture current state before changes)
    let snapshot = match create_snapshot(
        node_name,
        &ssh_config,
        &node.config_path,
        &node.service_name,
    )
    .await
    {
        Ok(s) => Some(s),
        Err(e) => {
            tracing::warn!(
                "Could not create rollback snapshot (first-time deploy?): {}",
                e
            );
            None
        }
    };

    // 3. Provision if requested
    if config.provision {
        let prov_result = provision_node(
            node_name,
            &ssh_config,
            config.config_content.as_deref(),
            &node.config_path,
            &node.service_name,
        )
        .await;

        if !prov_result.success {
            return DeployNodeResult {
                node_name: node_name.to_string(),
                status: DeployNodeStatus::Failed(format!(
                    "provisioning failed at step '{}': {}",
                    prov_result.last_step.label(),
                    prov_result.message
                )),
                health_before: Some(health_before),
                health_after: None,
                details: prov_result.message,
            };
        }
    } else {
        // 4. If not provisioning, just deploy the config file
        if let Some(ref content) = config.config_content {
            let mut client = match SshClient::connect(&ssh_config).await {
                Ok(c) => c,
                Err(e) => {
                    return DeployNodeResult {
                        node_name: node_name.to_string(),
                        status: DeployNodeStatus::Failed(format!("SSH connection failed: {}", e)),
                        health_before: Some(health_before),
                        health_after: None,
                        details: String::new(),
                    };
                }
            };

            // Transfer config
            if let Err(e) = client.write_file(&node.config_path, content).await {
                let _ = client.close().await;
                // Attempt rollback
                attempt_rollback(node_name, &snapshot, &ssh_config).await;
                return DeployNodeResult {
                    node_name: node_name.to_string(),
                    status: DeployNodeStatus::RolledBack,
                    health_before: Some(health_before),
                    health_after: None,
                    details: format!("config transfer failed: {}", e),
                };
            }

            // Restart service
            let restart_cmd = format!("systemctl restart {}", node.service_name);
            let restart_result = client.execute(&restart_cmd).await;
            let _ = client.close().await;

            match restart_result {
                Ok(r) if !r.success() => {
                    attempt_rollback(node_name, &snapshot, &ssh_config).await;
                    return DeployNodeResult {
                        node_name: node_name.to_string(),
                        status: DeployNodeStatus::RolledBack,
                        health_before: Some(health_before),
                        health_after: None,
                        details: format!(
                            "service restart failed (exit {}): {}",
                            r.exit_code,
                            r.stderr.trim()
                        ),
                    };
                }
                Err(e) => {
                    attempt_rollback(node_name, &snapshot, &ssh_config).await;
                    return DeployNodeResult {
                        node_name: node_name.to_string(),
                        status: DeployNodeStatus::RolledBack,
                        health_before: Some(health_before),
                        health_after: None,
                        details: format!("service restart command failed: {}", e),
                    };
                }
                _ => {}
            }
        }
    }

    // 5. Health check after deploy
    let health_after = check_node_health_detailed(
        node_name,
        &ssh_config,
        &node.service_name,
        &node.config_path,
    )
    .await;

    if !health_after.status.is_healthy() {
        // Health check failed — rollback
        let fail_reason = health_after.details.clone();
        attempt_rollback(node_name, &snapshot, &ssh_config).await;
        return DeployNodeResult {
            node_name: node_name.to_string(),
            status: DeployNodeStatus::RolledBack,
            health_before: Some(health_before),
            health_after: Some(health_after),
            details: format!("post-deploy health check failed: {}", fail_reason),
        };
    }

    DeployNodeResult {
        node_name: node_name.to_string(),
        status: DeployNodeStatus::Success,
        health_before: Some(health_before),
        health_after: Some(health_after),
        details: "deployment successful".into(),
    }
}

/// Attempt to roll back a node to its previous state.
/// Logs errors but does not propagate them (rollback is best-effort).
async fn attempt_rollback(
    node_name: &str,
    snapshot: &Option<RollbackSnapshot>,
    ssh_config: &SshConfig,
) {
    if let Some(ref snap) = snapshot {
        tracing::warn!("Rolling back '{}'...", node_name);
        if let Err(e) = rollback(snap, ssh_config).await {
            tracing::error!("Rollback failed for '{}': {}", node_name, e);
        } else {
            tracing::info!("Rollback successful for '{}'", node_name);
        }
    } else {
        tracing::warn!(
            "No rollback snapshot available for '{}' — cannot restore previous state",
            node_name
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deploy_node_status_labels() {
        assert_eq!(DeployNodeStatus::Success.label(), "success");
        assert_eq!(DeployNodeStatus::RolledBack.label(), "rolled back");
        assert_eq!(DeployNodeStatus::Failed("e".into()).label(), "failed");
        assert_eq!(DeployNodeStatus::Skipped("s".into()).label(), "skipped");
    }

    #[test]
    fn test_deploy_summary_counts() {
        let results = vec![
            DeployNodeResult {
                node_name: "a".into(),
                status: DeployNodeStatus::Success,
                health_before: None,
                health_after: None,
                details: String::new(),
            },
            DeployNodeResult {
                node_name: "b".into(),
                status: DeployNodeStatus::RolledBack,
                health_before: None,
                health_after: None,
                details: String::new(),
            },
            DeployNodeResult {
                node_name: "c".into(),
                status: DeployNodeStatus::Failed("err".into()),
                health_before: None,
                health_after: None,
                details: String::new(),
            },
            DeployNodeResult {
                node_name: "d".into(),
                status: DeployNodeStatus::Skipped("dry".into()),
                health_before: None,
                health_after: None,
                details: String::new(),
            },
        ];

        let summary = DeployOrchestrator::compute_summary(&results);
        assert_eq!(summary.total, 4);
        assert_eq!(summary.success, 1);
        assert_eq!(summary.rolled_back, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 1);
    }

    #[test]
    fn test_report_to_table() {
        let report = DeployReport {
            node_results: vec![DeployNodeResult {
                node_name: "gateway".into(),
                status: DeployNodeStatus::Success,
                health_before: None,
                health_after: None,
                details: "deployment successful".into(),
            }],
            summary: DeploySummary {
                total: 1,
                success: 1,
                rolled_back: 0,
                failed: 0,
                skipped: 0,
            },
        };
        let table = report.to_table();
        assert!(table.contains("success"));
        assert!(table.contains("gateway"));
    }
}
