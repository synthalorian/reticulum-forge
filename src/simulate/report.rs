//! Simulation report — formats metrics as terminal tables, JSON, and DOT graphs.

use crate::simulate::metrics::Metrics;
use serde::Serialize;
use std::time::Duration;

/// Complete simulation results.
#[derive(Debug, Clone, Serialize)]
pub struct SimulationReport {
    pub duration: Duration,
    pub metrics: Metrics,
    pub node_count: usize,
    pub link_count: usize,
}

impl SimulationReport {
    /// Render a terminal summary table using `console` styling.
    pub fn to_table(&self) -> String {
        let m = &self.metrics;
        let mut out = String::new();

        out.push_str("┌──────────────────────────────┬─────────────────────────────┐\n");
        out.push_str(&format!("│ {:<28} │ {:>27} │\n", "Metric", "Value"));
        out.push_str("├──────────────────────────────┼─────────────────────────────┤\n");
        out.push_str(&format!("│ {:<28} │ {:>27} │\n", "Nodes", self.node_count));
        out.push_str(&format!("│ {:<28} │ {:>27} │\n", "Links", self.link_count));
        out.push_str(&format!(
            "│ {:<28} │ {:>27} │\n",
            "Simulated duration",
            format_duration(self.duration)
        ));
        out.push_str(&format!(
            "│ {:<28} │ {:>27} │\n",
            "Packets sent", m.packets_sent
        ));
        out.push_str(&format!(
            "│ {:<28} │ {:>27} │\n",
            "Packets delivered", m.packets_delivered
        ));
        out.push_str(&format!(
            "│ {:<28} │ {:>27.1}% │\n",
            "Delivery rate",
            m.delivery_rate() * 100.0
        ));
        out.push_str(&format!(
            "│ {:<28} │ {:>27} │\n",
            "Packets lost", m.packets_lost
        ));
        out.push_str(&format!(
            "│ {:<28} │ {:>27.1}% │\n",
            "Loss rate",
            m.loss_rate() * 100.0
        ));
        out.push_str(&format!(
            "│ {:<28} │ {:>27.1} │\n",
            "Avg latency (ms)",
            m.avg_latency_ms()
        ));
        out.push_str(&format!(
            "│ {:<28} │ {:>27.1} │\n",
            "Min latency (ms)", m.min_latency_ms
        ));
        out.push_str(&format!(
            "│ {:<28} │ {:>27.1} │\n",
            "Max latency (ms)", m.max_latency_ms
        ));
        out.push_str(&format!(
            "│ {:<28} │ {:>27.1} │\n",
            "Avg hop count",
            m.avg_hops()
        ));
        out.push_str(&format!("│ {:<28} │ {:>27} │\n", "Min hops", m.min_hops));
        out.push_str(&format!("│ {:<28} │ {:>27} │\n", "Max hops", m.max_hops));
        out.push_str(&format!(
            "│ {:<28} │ {:>27} │\n",
            "Routes discovered", m.routes_discovered
        ));
        out.push_str(&format!(
            "│ {:<28} │ {:>27} │\n",
            "Announces propagated", m.announces_propagated
        ));
        out.push_str("└──────────────────────────────┴─────────────────────────────┘\n");

        out
    }

    /// Render the report as pretty JSON.
    pub fn to_json(&self) -> crate::error::ForgeResult<String> {
        serde_json::to_string_pretty(self).map_err(crate::error::ForgeError::SerdeJson)
    }

    /// Render a DOT (Graphviz) representation of the topology with metrics overlay.
    pub fn to_dot(&self) -> String {
        // Simple DOT output: just nodes (no edge data since we only have aggregate metrics)
        // In a full implementation, this would include link quality coloring.
        let mut dot = String::from("digraph reticulum_simulation {\n");
        dot.push_str("    rankdir=LR;\n");
        dot.push_str("    node [shape=point, style=filled, fillcolor=lightblue];\n");
        dot.push_str(&format!(
            "    label=\"Reticulum Simulation: {} nodes, {} links\\nDelivery: {:.1}%, Avg Latency: {:.1}ms, Avg Hops: {:.1}\";\n",
            self.node_count,
            self.link_count,
            self.metrics.delivery_rate() * 100.0,
            self.metrics.avg_latency_ms(),
            self.metrics.avg_hops()
        ));
        dot.push_str("    fontsize=14;\n");
        dot.push_str("    labelloc=t;\n");
        dot.push('}');
        dot
    }
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs >= 3600 {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    } else if secs >= 60 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulate::metrics::Metrics;
    use crate::simulate::node::Packet;
    use std::time::Duration;

    fn sample_report() -> SimulationReport {
        let mut m = Metrics::default();
        let mut p = Packet::new(1, "a", "b", "t");
        p.hops = 3;
        p.accumulated_latency = Duration::from_millis(150);
        m.record_delivered(&p);
        m.packets_sent = 10;
        m.routes_discovered = 8;

        SimulationReport {
            duration: Duration::from_secs(60),
            metrics: m,
            node_count: 5,
            link_count: 8,
        }
    }

    #[test]
    fn test_table_contains_metrics() {
        let report = sample_report();
        let table = report.to_table();
        assert!(table.contains("Delivery rate"));
        assert!(table.contains("Avg latency"));
        assert!(table.contains("Routes discovered"));
    }

    #[test]
    fn test_json_output() {
        let report = sample_report();
        let json = report.to_json().expect("JSON serialization should succeed");
        assert!(json.contains("node_count"));
        assert!(json.contains("link_count"));
        assert!(json.contains("packets_delivered"));
    }

    #[test]
    fn test_dot_output() {
        let report = sample_report();
        let dot = report.to_dot();
        assert!(dot.contains("digraph"));
        assert!(dot.contains("reticulum_simulation"));
        assert!(dot.contains("5 nodes"));
    }
}
