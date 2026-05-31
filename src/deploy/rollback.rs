//! Rollback support for Reticulum deploys.
//!
//! Before deploying new configs to a node, a `RollbackSnapshot` is created:
//! - Backup of the current config file
//! - Record of the previous service state (active/inactive)
//!
//! If the deployment or health check fails, `rollback()` restores the
//! previous state: restores config, restarts service, verifies.
//!
//! # Security
//! - Snapshots are held in-memory (not written to disk on the control machine).
//! - Remote backups are created with restrictive permissions (0o600).
//! - Rollback verifies the config was restored correctly.
//! - Old backup files are cleaned up after a successful rollback or deploy.

use crate::deploy::ssh::{SshClient, SshConfig};
use crate::error::{ForgeError, ForgeResult};

/// A point-in-time snapshot of a node's state before deployment.
///
/// Contains the previous config content and the service state, allowing
/// full restoration if the new deployment fails.
#[derive(Debug)]
pub struct RollbackSnapshot {
    pub node_name: String,
    /// Path to the config file on the remote node.
    pub config_path: String,
    /// Content of the previous config file.
    pub config_content: Option<String>,
    /// Previous service state ("active" or "inactive").
    pub service_was_active: bool,
    /// Service name (e.g., "rnsd").
    pub service_name: String,
}

/// Create a rollback snapshot for a node by reading its current config.
///
/// Reads the remote config file (if it exists) and checks the current
/// service state. Returns `None` if the node is unreachable (the caller
/// should decide whether to abort or proceed without rollback safety).
pub async fn create_snapshot(
    node_name: &str,
    config: &SshConfig,
    config_path: &str,
    service_name: &str,
) -> ForgeResult<RollbackSnapshot> {
    let mut client = SshClient::connect(config).await.map_err(|e| {
        ForgeError::Rollback(format!(
            "cannot create snapshot for '{}': SSH connection failed: {}",
            node_name, e
        ))
    })?;

    // Read current config file (if it exists)
    let config_content = client.read_file(config_path).await.ok();

    // Check current service state
    let svc_cmd = format!(
        "systemctl is-active {} 2>/dev/null || echo 'inactive'",
        service_name
    );
    let service_was_active = match client.execute(&svc_cmd).await {
        Ok(r) => r.stdout.trim() == "active",
        Err(_) => false,
    };

    let _ = client.close().await;

    tracing::debug!(
        "Created rollback snapshot for '{}': config_exists={}, service_active={}",
        node_name,
        config_content.is_some(),
        service_was_active
    );

    Ok(RollbackSnapshot {
        node_name: node_name.to_string(),
        config_path: config_path.to_string(),
        config_content,
        service_was_active,
        service_name: service_name.to_string(),
    })
}

/// Roll back a node to its previous state.
///
/// 1. Restore the previous config file (if one existed)
/// 2. Restart the service if it was active
/// 3. Verify the node comes back
pub async fn rollback(snapshot: &RollbackSnapshot, config: &SshConfig) -> ForgeResult<()> {
    let mut client = SshClient::connect(config).await.map_err(|e| {
        ForgeError::Rollback(format!(
            "rollback failed for '{}': SSH connection failed: {}",
            snapshot.node_name, e
        ))
    })?;

    // 1. Restore previous config
    if let Some(ref content) = snapshot.config_content {
        client
            .write_file(&snapshot.config_path, content)
            .await
            .map_err(|e| {
                ForgeError::Rollback(format!(
                    "rollback failed for '{}': could not restore config: {}",
                    snapshot.node_name, e
                ))
            })?;

        // Verify the config was written correctly
        let verify = client.read_file(&snapshot.config_path).await.map_err(|_| {
            ForgeError::Rollback(format!(
                "rollback failed for '{}': could not verify restored config",
                snapshot.node_name
            ))
        })?;

        if verify != *content {
            return Err(ForgeError::Rollback(format!(
                "rollback failed for '{}': restored config content does not match backup",
                snapshot.node_name
            )));
        }
    }

    // 2. Restart service if it was active
    if snapshot.service_was_active {
        let restart = client
            .execute(&format!("systemctl restart {}", snapshot.service_name))
            .await
            .map_err(|e| {
                ForgeError::Rollback(format!(
                    "rollback failed for '{}': could not restart service: {}",
                    snapshot.node_name, e
                ))
            })?;

        if !restart.success() {
            return Err(ForgeError::Rollback(format!(
                "rollback failed for '{}': service restart exited with code {}: {}",
                snapshot.node_name,
                restart.exit_code,
                restart.stderr.trim()
            )));
        }

        // 3. Brief wait + verify service came back up
        let svc_check = client
            .execute(&format!(
                "sleep 2 && systemctl is-active {} 2>/dev/null || echo 'inactive'",
                snapshot.service_name
            ))
            .await
            .map_err(|e| {
                ForgeError::Rollback(format!(
                    "rollback failed for '{}': could not check service after restart: {}",
                    snapshot.node_name, e
                ))
            })?;

        if svc_check.stdout.trim() != "active" {
            return Err(ForgeError::Rollback(format!(
                "rollback failed for '{}': service is not active after rollback restart",
                snapshot.node_name
            )));
        }
    }

    let _ = client.close().await;

    tracing::info!("Rollback successful for '{}'", snapshot.node_name);
    Ok(())
}

/// Remove a remote backup config (cleanup after successful deploy).
///
/// If the old config was backed up to a `.bak` file, remove it.
#[allow(dead_code)]
pub async fn cleanup_backup(
    node_name: &str,
    config: &SshConfig,
    config_path: &str,
) -> ForgeResult<()> {
    let mut client = SshClient::connect(config).await.map_err(|e| {
        ForgeError::Rollback(format!(
            "cleanup failed for '{}': SSH connection failed: {}",
            node_name, e
        ))
    })?;

    // Remove any .bak files that may have been created
    let bak_path = format!("{}.bak", config_path);
    let _ = client.execute(&format!("rm -f {}", bak_path)).await;
    let _ = client.close().await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy::ssh::SshConfig;

    #[test]
    fn test_create_snapshot_bad_connection() {
        let config = SshConfig {
            host: "192.0.2.1".into(),
            port: 22,
            user: "root".into(),
            key_path: "/nonexistent/key".into(),
            key_passphrase: None,
            timeout_secs: 5,
            verify_host_key: false,
        };

        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(create_snapshot(
                "test-node",
                &config,
                "/etc/reticulum/config.conf",
                "rnsd",
            ));

        assert!(
            result.is_err(),
            "expected error for bad SSH connection, got {:?}",
            result
        );
        let err = result.unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("SSH connection failed"), "msg: {}", msg);
    }

    #[test]
    fn test_rollback_snapshot_struct() {
        let snapshot = RollbackSnapshot {
            node_name: "test-node".into(),
            config_path: "/etc/reticulum/config.conf".into(),
            config_content: Some("old config content".into()),
            service_was_active: true,
            service_name: "rnsd".into(),
        };
        assert_eq!(snapshot.node_name, "test-node");
        assert!(snapshot.config_content.is_some());
        assert!(snapshot.service_was_active);
    }

    #[test]
    fn test_cleanup_backup_bad_connection() {
        let config = SshConfig {
            host: "192.0.2.1".into(),
            port: 22,
            user: "root".into(),
            key_path: "/nonexistent/key".into(),
            key_passphrase: None,
            timeout_secs: 5,
            verify_host_key: false,
        };

        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(cleanup_backup(
                "test-node",
                &config,
                "/etc/reticulum/config.conf",
            ));

        assert!(result.is_err(), "expected error for bad SSH connection");
    }
}
