//! `forge deploy` — push configs to remote nodes via SSH.
//!
//! Orchestrates rolling deployment across a fleet of Reticulum nodes:
//! - Inventory management via `nodes.toml`
//! - SSH-based config transfer with russh
//! - Rolling deployment with health checks between nodes
//! - Automatic rollback on failure
//! - Full provisioning mode (`--provision`)
//! - Parallel deployment (`--concurrency N`)
//! - Dry-run mode (`--dry-run`)
//!
//! # Security
//! - Dry-run never touches remote machines.
//! - Rollback snapshots created before any config change.
//! - SSH key-only auth (password auth not supported).
//! - Inventory paths validated for directory traversal.
//! - Concurrent operations bounded by user-specified limit.
//! - Host key verification is enforced at the SSH layer.

use crate::deploy::DeployConfig;
use crate::deploy::DeployOrchestrator;
use crate::error::{ForgeError, ForgeResult};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Maximum deploy concurrency to prevent resource exhaustion.
const MAX_CONCURRENCY: usize = 32;

/// Run the deploy command.
#[expect(clippy::too_many_arguments)]
pub fn execute(
    inventory_path: &str,
    dry_run: bool,
    concurrency: usize,
    provision: bool,
    tag: Option<&str>,
    config_content: Option<String>,
    format: &str,
    output: Option<&Path>,
) -> ForgeResult<()> {
    // ---- validate inventory path ----
    if inventory_path.contains("..") {
        return Err(ForgeError::Validation(
            "inventory path must not contain '..' (directory traversal)".into(),
        ));
    }

    // ---- validate concurrency ----
    if concurrency == 0 {
        return Err(ForgeError::Validation(
            "concurrency must be at least 1".into(),
        ));
    }
    if concurrency > MAX_CONCURRENCY {
        return Err(ForgeError::Validation(format!(
            "concurrency must not exceed {}",
            MAX_CONCURRENCY
        )));
    }

    // ---- validate output format ----
    let fmt = format.to_lowercase();
    match fmt.as_str() {
        "table" | "json" => {}
        _ => {
            return Err(ForgeError::Cli(format!(
                "unsupported output format '{}'. Use: table or json",
                format
            )));
        }
    }

    // ---- progress ----
    let pb = ProgressBar::new(2);
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("static template is valid")
            .tick_strings(&[
                "▹▹▹▹▹",
                "▸▹▹▹▹",
                "▹▸▹▹▹",
                "▹▹▸▹▹",
                "▹▹▹▸▹",
                "▹▹▹▹▸",
                "▪▪▪▪▪",
            ]),
    );

    // 1. Load inventory and create orchestrator
    pb.set_message("Loading inventory...");
    let deploy_config = DeployConfig {
        inventory_path: inventory_path.to_string(),
        dry_run,
        concurrency,
        provision,
        tag_filter: tag.map(|s| s.to_string()),
        config_content,
        timeout_secs: 30,
    };

    let orchestrator = DeployOrchestrator::new(deploy_config).inspect_err(|_| {
        pb.finish_with_message("✖ Inventory load failed");
    })?;
    pb.inc(1);

    // 2. Execute deployment
    let status_msg = if dry_run {
        "Dry-run mode — no changes will be made"
    } else if provision {
        "Deploying with provisioning..."
    } else {
        "Deploying configurations..."
    };
    pb.set_message(status_msg);

    let rt = tokio::runtime::Runtime::new().map_err(ForgeError::Io)?;
    let report = rt
        .block_on(async { orchestrator.deploy().await })
        .inspect_err(|_| {
            pb.finish_with_message("✖ Deploy failed");
        })?;
    pb.inc(1);

    // 3. Finish progress
    let finish_icon = if report.summary.failed == 0 && report.summary.rolled_back == 0 {
        style("✔").green()
    } else if report.summary.failed > 0 || report.summary.rolled_back > 0 {
        style("⚠").yellow()
    } else {
        style("✔").green()
    };

    pb.finish_with_message(format!(
        "{} Deploy complete: {} success, {} rolled back, {} failed, {} skipped",
        finish_icon,
        report.summary.success,
        report.summary.rolled_back,
        report.summary.failed,
        report.summary.skipped,
    ));

    // 4. Format output
    let output_content: String = match fmt.as_str() {
        "table" => report.to_table(),
        "json" => report.to_json()?,
        _ => unreachable!(),
    };

    // 5. Write or print
    if let Some(out_path) = output {
        // Security: validate output path (no directory traversal)
        out_path
            .file_name()
            .ok_or_else(|| ForgeError::Validation("output path must be a file name".into()))?;

        if out_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_string_lossy()
            .contains("..")
        {
            return Err(ForgeError::Validation(
                "output path must not contain '..' (directory traversal)".into(),
            ));
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(out_path, &output_content).map_err(ForgeError::Io)?;
            std::fs::set_permissions(out_path, std::fs::Permissions::from_mode(0o644))
                .map_err(ForgeError::Io)?;
        }
        #[cfg(not(unix))]
        {
            std::fs::write(out_path, &output_content).map_err(ForgeError::Io)?;
        }

        println!("  {} {}", style("📄").bold(), out_path.display());
    } else {
        println!("{}", output_content);
    }

    // 6. Return error exit code if any deploys failed
    if report.summary.failed > 0 || report.summary.rolled_back > 0 {
        Err(ForgeError::Deploy(format!(
            "{} node(s) failed and {} node(s) were rolled back",
            report.summary.failed, report.summary.rolled_back
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_concurrency_value() {
        assert_eq!(MAX_CONCURRENCY, 32);
    }
}
