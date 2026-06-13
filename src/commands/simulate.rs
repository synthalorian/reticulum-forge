//! `forge simulate` — run a virtual Reticulum network simulation.
//!
//! Generates a network topology, runs announce propagation and data traffic
//! simulation, and outputs metrics as a terminal table, JSON, or DOT graph.

use crate::error::{ForgeError, ForgeResult};
use crate::simulate::engine::{SimConfig, SimEngine};
use crate::simulate::report::SimulationReport;
use crate::simulate::topology::{generate, TopologyConfig, TopologyType};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::Duration;

/// Run the simulation command.
#[expect(clippy::too_many_arguments)]
///
/// * `node_count` — number of virtual nodes.
/// * `topology_name` — "mesh", "star", "ring", or "chain".
/// * `duration_str` — e.g. "30s", "5m", "1h".
/// * `link_quality` — "excellent", "good", "moderate", "poor".
/// * `data_interval_str` — e.g. "1s", "100ms" for data packet generation interval.
/// * `packets_per_cycle` — packets to send per node pair per interval.
/// * `format` — "table", "json", or "dot".
/// * `output` — optional file path (default: stdout).
pub fn execute(
    node_count: usize,
    topology_name: &str,
    duration_str: &str,
    link_quality: &str,
    data_interval_str: Option<&str>,
    packets_per_cycle: Option<usize>,
    format: &str,
    output: Option<&Path>,
) -> ForgeResult<()> {
    // ---- validate topology type ----
    let topo_type = TopologyType::from_str(topology_name).ok_or_else(|| {
        ForgeError::Cli(format!(
            "unknown topology '{}'. Valid types: {}",
            topology_name,
            TopologyType::all_names().join(", ")
        ))
    })?;

    // ---- validate link quality ----
    let valid_qualities = &["excellent", "good", "moderate", "poor"];
    if !valid_qualities.contains(&link_quality.to_lowercase().as_str()) {
        return Err(ForgeError::Cli(format!(
            "unknown link quality '{}'. Valid: {}",
            link_quality,
            valid_qualities.join(", ")
        )));
    }

    // ---- validate node count ----
    if node_count < 2 {
        return Err(ForgeError::Cli(
            "simulation requires at least 2 nodes".into(),
        ));
    }

    // ---- parse duration ----
    let duration = parse_duration(duration_str).ok_or_else(|| {
        ForgeError::Cli(format!(
            "invalid duration '{}'. Use e.g. 30s, 5m, 1h",
            duration_str
        ))
    })?;

    // ---- validate output format ----
    let format = format.to_lowercase();
    match format.as_str() {
        "table" | "json" | "dot" => {}
        _ => {
            return Err(ForgeError::Cli(format!(
                "unsupported output format '{}'. Use: table, json, or dot",
                format
            )));
        }
    }

    // ---- progress ----
    let pb = ProgressBar::new(3);
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

    // 1. Generate topology
    pb.set_message(format!(
        "Generating {}-node {} topology...",
        node_count, topology_name
    ));
    let topo_config = TopologyConfig {
        node_count,
        topology_type: topo_type,
        link_quality: link_quality.to_string(),
        partial_degree: None,
    };
    let topology = generate(&topo_config);
    pb.inc(1);

    // 2. Run simulation
    pb.set_message("Running simulation...");
    let data_interval = data_interval_str
        .and_then(parse_duration)
        .unwrap_or_else(|| Duration::from_secs(5));
    let sim_config = SimConfig {
        duration,
        data_interval,
        packets_per_cycle: packets_per_cycle.unwrap_or(3),
        ..Default::default()
    };
    let mut engine = SimEngine::new(topology, sim_config);

    let rt = tokio::runtime::Runtime::new().map_err(ForgeError::Io)?;
    let report: SimulationReport = rt.block_on(async { engine.run().await });
    pb.inc(1);

    // 3. Format output
    pb.set_message("Formatting results...");
    let output_content: String = match format.as_str() {
        "table" => report.to_table(),
        "json" => report.to_json()?,
        "dot" => report.to_dot(),
        _ => unreachable!(),
    };
    pb.inc(1);

    pb.finish_with_message(format!("{} Simulation complete", style("✔").green()));

    // 4. Write or print
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

/// Parse a duration string like "30s", "5m", "1h" into a Duration.
fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim().to_lowercase();
    if let Some(rest) = s.strip_suffix('h') {
        let hours: f64 = rest.parse().ok()?;
        Some(Duration::from_secs_f64(hours * 3600.0))
    } else if let Some(rest) = s.strip_suffix('m') {
        let mins: f64 = rest.parse().ok()?;
        Some(Duration::from_secs_f64(mins * 60.0))
    } else if let Some(rest) = s.strip_suffix('s') {
        let secs: f64 = rest.parse().ok()?;
        Some(Duration::from_secs_f64(secs))
    } else {
        // Plain number = seconds
        let secs: f64 = s.parse().ok()?;
        Some(Duration::from_secs_f64(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s"), Some(Duration::from_secs(30)));
        assert_eq!(parse_duration("5m"), Some(Duration::from_secs(300)));
        assert_eq!(parse_duration("1h"), Some(Duration::from_secs(3600)));
        assert_eq!(parse_duration("0.5s"), Some(Duration::from_millis(500)));
        assert_eq!(parse_duration("60"), Some(Duration::from_secs(60)));
        assert!(parse_duration("abc").is_none());
    }
}
