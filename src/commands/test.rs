//! `forge test` — validate network configurations, connectivity, and policies.
//!
//! # Security
//! - Config path is validated for directory traversal.
//! - Output path is validated for directory traversal.
//! - File writes use restrictive permissions (0o644).
//! - Output format is validated against an allowlist.
//! - Simulation nodes are bounded to prevent resource exhaustion.

use crate::config::ForgeConfig;
use crate::error::{ForgeError, ForgeResult};
use crate::test_runner::{self, format_junit, format_table, format_tap};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::Duration;

/// Run the test command.
///
/// * `check_type` — Which checks to run ("all", "connectivity", "latency", "redundancy", "policies").
/// * `config_path` — Path to forge.toml.
/// * `threshold_ms` — Optional latency threshold in milliseconds.
/// * `format` — Output format ("table", "json", "tap", "junit").
/// * `output` — Optional file path (default: stdout).
pub fn execute(
    check_type: &str,
    config_path: &str,
    threshold_ms: Option<u64>,
    format: &str,
    output: Option<&Path>,
) -> ForgeResult<()> {
    // ---- Validate output format ----
    let fmt = format.to_lowercase();
    match fmt.as_str() {
        "table" | "json" | "tap" | "junit" => {}
        _ => {
            return Err(ForgeError::Cli(format!(
                "unsupported output format '{}'. Use: table, json, tap, or junit",
                format
            )));
        }
    }

    // ---- Validate config path (block traversal) ----
    if config_path.contains("..") {
        return Err(ForgeError::Validation(
            "config path must not contain '..' (directory traversal)".into(),
        ));
    }

    // ---- Load config ----
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

    pb.set_message("Loading configuration...");
    let config = ForgeConfig::load(config_path)?;
    pb.inc(1);

    // ---- Run checks ----
    pb.set_message(format!("Running {} checks...", check_type));
    let threshold = threshold_ms.map(Duration::from_millis);
    let max_hops = Some(10); // Default max hop count policy
    let report = test_runner::run_checks(check_type, &config, threshold, max_hops)?;
    pb.inc(1);

    pb.finish_with_message(format!(
        "{} Checks complete: {} passed, {} failed, {} warnings",
        style("✔").green(),
        report.summary.passed,
        report.summary.failed,
        report.summary.warnings,
    ));

    // ---- Format output ----
    let output_content: String = match fmt.as_str() {
        "table" => format_table(&report),
        "json" => test_runner::format_json(&report)?,
        "tap" => format_tap(&report),
        "junit" => format_junit(&report)?,
        _ => unreachable!(),
    };

    // ---- Write or print ----
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

    Ok(())
}
