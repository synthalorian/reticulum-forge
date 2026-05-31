//! Remote health checks for deployed Reticulum nodes.
//!
//! Verifies node health by SSH-ing in and checking:
//! - Basic connectivity (uptime)
//! - Reticulum process running (pgrep)
//! - Config file exists (test -f)
//! - Service status (systemctl is-active)
//!
//! # Security
//! - Commands use safe, predictable calls (no user-controlled interpolation).
//! - Connection errors don't panic — they produce `Offline` status.
//! - Timeouts configured via `SshConfig::timeout_secs`.

use crate::deploy::ssh::{SshClient, SshConfig};
use serde::Serialize;

/// Health status of a single node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum HealthStatus {
    /// Node is fully operational.
    Healthy,
    /// Node is reachable but has issues.
    Degraded(String),
    /// Node is unreachable via SSH.
    Offline(String),
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    pub fn is_degraded(&self) -> bool {
        matches!(self, HealthStatus::Degraded(_))
    }

    pub fn is_offline(&self) -> bool {
        matches!(self, HealthStatus::Offline(_))
    }

    #[allow(dead_code)]
    pub fn label(&self) -> &str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded(_) => "degraded",
            HealthStatus::Offline(_) => "offline",
        }
    }
}

/// Result of a health check run against a single node.
#[derive(Debug, Clone, Serialize)]
pub struct HealthCheckResult {
    pub node_name: String,
    pub status: HealthStatus,
    pub details: String,
}

/// Run a detailed health check against a single node.
///
/// Accepts node inventory fields (`service_name`, `config_path`) for
/// detailed checks beyond basic connectivity.
pub async fn check_node_health_detailed(
    node_name: &str,
    config: &SshConfig,
    service_name: &str,
    config_path: &str,
) -> HealthCheckResult {
    let mut client = match SshClient::connect(config).await {
        Ok(c) => c,
        Err(e) => {
            return HealthCheckResult {
                node_name: node_name.to_string(),
                status: HealthStatus::Offline(format!("SSH connection failed: {}", e)),
                details: String::new(),
            };
        }
    };

    let mut issues: Vec<String> = Vec::new();

    // 1. Check config file exists
    let config_cmd = format!("test -f {}", config_path);
    match client.execute(&config_cmd).await {
        Ok(r) if !r.success() => {
            issues.push(format!("config file '{}' not found", config_path));
        }
        Err(e) => {
            issues.push(format!("config check failed: {}", e));
        }
        _ => {}
    }

    // 2. Check systemd service status
    let svc_cmd = format!(
        "systemctl is-active {} 2>/dev/null || echo 'inactive'",
        service_name
    );
    match client.execute(&svc_cmd).await {
        Ok(r) => {
            let status = r.stdout.trim();
            if status != "active" {
                issues.push(format!("service '{}' status: {}", service_name, status));
            }
        }
        Err(e) => {
            issues.push(format!("service check failed: {}", e));
        }
    }

    // 3. Check Reticulum process
    let ps_cmd = "pgrep -x rnsd 2>/dev/null || pgrep -x reticulum 2>/dev/null || echo 'no_process'";
    match client.execute(ps_cmd).await {
        Ok(r) if r.stdout.trim() == "no_process" => {
            issues.push("no Reticulum process (rnsd/reticulum) running".into());
        }
        Err(e) => {
            issues.push(format!("process check failed: {}", e));
        }
        _ => {}
    }

    let _ = client.close().await;

    let status = if issues.is_empty() {
        HealthStatus::Healthy
    } else {
        HealthStatus::Degraded(issues.join("; "))
    };

    HealthCheckResult {
        node_name: node_name.to_string(),
        status,
        details: if issues.is_empty() {
            format!("node '{}' is fully operational", node_name)
        } else {
            format!("node '{}' issues: {}", node_name, issues.join("; "))
        },
    }
}

/// Quick connectivity check — SSH + echo.
///
/// Lighter weight than the detailed check, useful for pre-deploy validation.
#[allow(dead_code)]
pub async fn check_node_reachable(node_name: &str, config: &SshConfig) -> HealthCheckResult {
    let mut client = match SshClient::connect(config).await {
        Ok(c) => c,
        Err(e) => {
            return HealthCheckResult {
                node_name: node_name.to_string(),
                status: HealthStatus::Offline(format!("SSH connection failed: {}", e)),
                details: String::new(),
            };
        }
    };

    let result = client.execute("echo 'reachable'").await;
    let _ = client.close().await;

    match result {
        Ok(r) if r.stdout.trim() == "reachable" => HealthCheckResult {
            node_name: node_name.to_string(),
            status: HealthStatus::Healthy,
            details: "node is reachable via SSH".into(),
        },
        Ok(r) => HealthCheckResult {
            node_name: node_name.to_string(),
            status: HealthStatus::Degraded(format!("unexpected response: {}", r.stdout.trim())),
            details: String::new(),
        },
        Err(e) => HealthCheckResult {
            node_name: node_name.to_string(),
            status: HealthStatus::Offline(format!("command failed: {}", e)),
            details: String::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_labels() {
        assert_eq!(HealthStatus::Healthy.label(), "healthy");
        assert_eq!(HealthStatus::Degraded("test".into()).label(), "degraded");
        assert_eq!(HealthStatus::Offline("test".into()).label(), "offline");
    }

    #[test]
    fn test_health_status_is_healthy() {
        assert!(HealthStatus::Healthy.is_healthy());
        assert!(!HealthStatus::Degraded("cpu high".into()).is_healthy());
        assert!(!HealthStatus::Offline("timeout".into()).is_healthy());
    }

    #[test]
    fn test_health_result_offline_on_bad_key() {
        let config = SshConfig {
            host: "192.0.2.1".into(),
            port: 22,
            user: "root".into(),
            key_path: "/nonexistent/bad_key_file".into(),
            key_passphrase: None,
            timeout_secs: 5,
            verify_host_key: false,
        };

        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(check_node_health_detailed(
                "test-node",
                &config,
                "rnsd",
                "/etc/reticulum/config.conf",
            ));

        assert_eq!(result.node_name, "test-node");
        assert!(
            matches!(result.status, HealthStatus::Offline(_)),
            "expected Offline (bad key), got {:?}",
            result.status
        );
    }
}
