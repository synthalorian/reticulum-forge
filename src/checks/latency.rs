use crate::checks::{CheckCategory, CheckResult};
use crate::simulate::engine::{SimConfig, SimEngine};
use crate::simulate::topology::{generate, TopologyConfig, TopologyType};
use std::time::Duration;

pub fn check_latency(
    node_count: usize,
    topology_type: TopologyType,
    link_quality: &str,
    threshold: Option<Duration>,
) -> Vec<CheckResult> {
    let mut results = Vec::new();

    if node_count < 2 {
        results.push(CheckResult::error(
            CheckCategory::Latency,
            "minimum-nodes",
            "latency check requires at least 2 nodes",
        ));
        return results;
    }

    let topo_config = TopologyConfig {
        node_count,
        topology_type,
        link_quality: link_quality.to_string(),
        partial_degree: None,
    };
    let topology = generate(&topo_config);

    let sim_config = SimConfig {
        duration: Duration::from_secs(5),
        ..Default::default()
    };
    let mut engine = SimEngine::new(topology, sim_config);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let report = rt.block_on(async { engine.run().await });

    let metrics = &report.metrics;
    let avg_latency = metrics.avg_latency_ms();
    let max_latency = metrics.max_latency_ms;
    let min_latency = metrics.min_latency_ms;

    results.push(CheckResult::pass(
        CheckCategory::Latency,
        "avg-latency",
        &format!("average latency: {:.1}ms", avg_latency),
    ));
    results.push(CheckResult::pass(
        CheckCategory::Latency,
        "min-latency",
        &format!("minimum latency: {:.1}ms", min_latency),
    ));
    results.push(CheckResult::pass(
        CheckCategory::Latency,
        "max-latency",
        &format!("maximum latency: {:.1}ms", max_latency),
    ));

    if let Some(thresh) = threshold {
        let avg_dur = Duration::from_secs_f64(avg_latency / 1000.0);
        if avg_dur > thresh {
            results.push(CheckResult::fail(
                CheckCategory::Latency,
                "latency-threshold",
                &format!(
                    "average latency {:.1}ms exceeds threshold of {:.0}ms",
                    avg_latency,
                    thresh.as_millis()
                ),
            ));
        } else {
            results.push(CheckResult::pass(
                CheckCategory::Latency,
                "latency-threshold",
                &format!(
                    "average latency {:.1}ms is within threshold of {:.0}ms",
                    avg_latency,
                    thresh.as_millis()
                ),
            ));
        }
    }

    let delivery_pct = metrics.delivery_rate() * 100.0;
    if delivery_pct < 100.0 {
        results.push(CheckResult::warn(
            CheckCategory::Latency,
            "packet-delivery",
            &format!("packet delivery rate is {:.1}%", delivery_pct),
        ));
    } else {
        results.push(CheckResult::pass(
            CheckCategory::Latency,
            "packet-delivery",
            "100% packet delivery",
        ));
    }

    let avg_hops = metrics.avg_hops();
    results.push(CheckResult::pass(
        CheckCategory::Latency,
        "avg-hop-count",
        &format!("average hop count: {:.1}", avg_hops),
    ));

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::CheckStatus;

    #[test]
    fn test_latency_small_mesh() {
        let results = check_latency(3, TopologyType::Mesh, "excellent", None);
        let errors: Vec<_> = results
            .iter()
            .filter(|r| r.status == CheckStatus::Error)
            .collect();
        assert!(errors.is_empty(), "no errors expected: {:?}", errors);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_latency_less_than_two_nodes() {
        let results = check_latency(1, TopologyType::Mesh, "excellent", None);
        let error = results.iter().find(|r| r.name == "minimum-nodes").unwrap();
        assert_eq!(error.status, CheckStatus::Error);
    }

    #[test]
    fn test_latency_threshold_triggered() {
        let results = check_latency(
            3,
            TopologyType::Mesh,
            "excellent",
            Some(Duration::from_micros(1)),
        );
        let tr = results
            .iter()
            .find(|r| r.name == "latency-threshold")
            .unwrap();
        assert_eq!(tr.status, CheckStatus::Fail);
    }
}
