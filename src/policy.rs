//! Network policy checks for `forge test`.
//!
//! Evaluates encryption requirements, single points of failure (cut vertices),
//! and maximum hop count policies against a simulated network topology.

use crate::checks::redundancy;
use crate::checks::{CheckCategory, CheckResult};
use crate::config::ForgeConfig;
use crate::simulate::engine::{SimConfig, SimEngine};
use crate::simulate::topology::{generate, TopologyConfig, TopologyType};
use std::time::Duration;

#[allow(dead_code)]
pub const POLICY_ENCRYPTED_LINKS: &str = "encrypted-links";
#[allow(dead_code)]
pub const POLICY_NO_CUT_VERTICES: &str = "no-cut-vertices";
#[allow(dead_code)]
pub const POLICY_MAX_HOP_COUNT: &str = "max-hop-count";

pub fn check_policies(
    config: &ForgeConfig,
    topology_edges: &[(&str, &str)],
    max_hops: Option<u32>,
) -> Vec<CheckResult> {
    let mut results = Vec::new();
    results.push(check_encrypted_links(config));
    results.extend(check_cut_vertices(topology_edges));
    if let Some(hops) = max_hops {
        results.push(check_max_hops(config, topology_edges, hops));
    }
    results
}

fn check_encrypted_links(config: &ForgeConfig) -> CheckResult {
    let transport = config
        .network
        .as_ref()
        .and_then(|n| n.transport.as_deref())
        .unwrap_or("unknown");
    match transport {
        "udp" => CheckResult::warn(
            CheckCategory::Policy, "encrypted-links",
            "transport is 'udp' — Reticulum's UDP transport does not natively encrypt. Consider TCP with TLS.",
        ),
        "tcp" => CheckResult::pass(
            CheckCategory::Policy, "encrypted-links",
            "transport is 'tcp' — supports encryption. Ensure require_encryption=yes on all interfaces.",
        ),
        other => CheckResult::warn(
            CheckCategory::Policy, "encrypted-links",
            &format!("transport '{}' encryption status unknown", other),
        ),
    }
}

pub(crate) fn check_cut_vertices(topology_edges: &[(&str, &str)]) -> Vec<CheckResult> {
    redundancy::check_redundancy(topology_edges)
}

fn check_max_hops(
    config: &ForgeConfig,
    _topology_edges: &[(&str, &str)],
    max_hops: u32,
) -> CheckResult {
    let topo_str = config.project.topology.as_deref().unwrap_or("mesh");
    let topo_type = TopologyType::from_str(topo_str).unwrap_or(TopologyType::Mesh);

    let node_count = 10;
    let topo_config = TopologyConfig {
        node_count,
        topology_type: topo_type,
        link_quality: "good".to_string(),
        partial_degree: None,
    };
    let topology = generate(&topo_config);

    let sim_config = SimConfig {
        duration: Duration::from_secs(5),
        ..Default::default()
    };
    let mut engine = SimEngine::new(topology, sim_config);
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            return CheckResult::error(
                CheckCategory::Policy,
                "max-hop-count",
                &format!("failed to create tokio runtime: {}", e),
            );
        }
    };
    let report = rt.block_on(async { engine.run().await });

    let avg_hops = report.metrics.avg_hops();
    let max_hops_observed = report.metrics.max_hops;

    if max_hops_observed > max_hops {
        CheckResult::fail(
            CheckCategory::Policy,
            "max-hop-count",
            &format!(
                "network diameter of {} hops exceeds maximum of {} hops (simulated with {} nodes)",
                max_hops_observed, max_hops, node_count
            ),
        )
        .with_details(&format!("Average hop count: {:.1}", avg_hops))
    } else {
        CheckResult::pass(
            CheckCategory::Policy,
            "max-hop-count",
            &format!(
                "network diameter of {} hops is within maximum of {} hops",
                max_hops_observed, max_hops
            ),
        )
    }
}

pub fn config_to_edges(config: &ForgeConfig) -> Vec<(String, String)> {
    let topo_str = config.project.topology.as_deref().unwrap_or("mesh");
    let topo_type = TopologyType::from_str(topo_str).unwrap_or(TopologyType::Mesh);

    let topo_config = TopologyConfig {
        node_count: 8,
        topology_type: topo_type,
        link_quality: "good".to_string(),
        partial_degree: None,
    };
    let topology = generate(&topo_config);

    let project_name = &config.project.name;
    topology
        .links
        .iter()
        .map(|link| {
            let src = format!("{}-{}", project_name, link.node_a);
            let dst = format!("{}-{}", project_name, link.node_b);
            (src, dst)
        })
        .collect()
}
