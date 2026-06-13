//! Connectivity check — validates that all nodes in a topology can reach each other.
//!
//! Uses petgraph to analyze graph connectivity and find disconnected components.

use crate::checks::{CheckCategory, CheckResult};
use petgraph::algo::has_path_connecting;
use petgraph::graph::{Graph, NodeIndex};
use petgraph::Undirected;
use std::collections::HashMap;

/// Run connectivity checks against a topology graph.
pub fn check_connectivity(topology_edges: &[(&str, &str)]) -> Vec<CheckResult> {
    let mut results = Vec::new();
    let (graph, node_map, node_names) = build_graph(topology_edges);

    if graph.node_count() == 0 {
        results.push(CheckResult::error(
            CheckCategory::Connectivity,
            "empty-network",
            "network has no nodes defined",
        ));
        return results;
    }

    let mut isolated: Vec<String> = Vec::new();
    for name in &node_names {
        if let Some(&idx) = node_map.get(name.as_str()) {
            if graph.neighbors(idx).count() == 0 {
                isolated.push(name.clone());
            }
        }
    }

    if !isolated.is_empty() {
        results.push(
            CheckResult::fail(
                CheckCategory::Connectivity,
                "isolated-nodes",
                &format!("found {} isolated nodes", isolated.len()),
            )
            .with_details(&format!("Isolated nodes: {}", isolated.join(", "))),
        );
    } else {
        results.push(CheckResult::pass(
            CheckCategory::Connectivity,
            "isolated-nodes",
            "no isolated nodes found",
        ));
    }

    let components = find_connected_components(&graph, &node_map, &node_names);
    if components.len() > 1 {
        results.push(
            CheckResult::fail(
                CheckCategory::Connectivity,
                "disconnected-components",
                &format!(
                    "network has {} disconnected components (expected 1)",
                    components.len()
                ),
            )
            .with_details(&format!(
                "Component sizes: {}",
                components
                    .iter()
                    .map(|c| format!("{} nodes", c.len()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        );
    } else {
        results.push(CheckResult::pass(
            CheckCategory::Connectivity,
            "disconnected-components",
            "all nodes are connected in a single component",
        ));
    }

    results.push(CheckResult::pass(
        CheckCategory::Connectivity,
        "node-count",
        &format!("network has {} nodes", graph.node_count()),
    ));

    results.push(CheckResult::pass(
        CheckCategory::Connectivity,
        "edge-count",
        &format!("network has {} links", graph.edge_count()),
    ));

    if graph.node_count() <= 20 && graph.node_count() > 1 {
        let indices: Vec<NodeIndex> = node_names.iter().map(|n| node_map[n.as_str()]).collect();
        let mut unreachable_pairs = Vec::new();

        for (i, &a) in indices.iter().enumerate() {
            for (j, &b) in indices.iter().enumerate().skip(i + 1) {
                if !has_path_connecting(&graph, a, b, None) {
                    unreachable_pairs.push(format!("{} <-> {}", node_names[i], node_names[j]));
                }
            }
        }

        if unreachable_pairs.is_empty() {
            results.push(CheckResult::pass(
                CheckCategory::Connectivity,
                "pair-connectivity",
                &format!(
                    "all {} node pairs can reach each other",
                    graph.node_count() * (graph.node_count() - 1) / 2
                ),
            ));
        } else {
            results.push(
                CheckResult::fail(
                    CheckCategory::Connectivity,
                    "pair-connectivity",
                    &format!(
                        "{} node pairs cannot reach each other",
                        unreachable_pairs.len()
                    ),
                )
                .with_details(&format!(
                    "Unreachable pairs: {}",
                    unreachable_pairs.join(", ")
                )),
            );
        }
    }

    results
}

fn build_graph(
    edges: &[(&str, &str)],
) -> (
    Graph<(), (), Undirected>,
    HashMap<String, NodeIndex>,
    Vec<String>,
) {
    let mut graph = Graph::<(), (), Undirected>::new_undirected();
    let mut node_map: HashMap<String, NodeIndex> = HashMap::new();
    let mut node_names: Vec<String> = Vec::new();

    for (a, b) in edges {
        let na = *node_map
            .entry(a.to_string())
            .or_insert_with(|| graph.add_node(()));
        let nb = *node_map
            .entry(b.to_string())
            .or_insert_with(|| graph.add_node(()));
        graph.add_edge(na, nb, ());
    }

    let mut seen = std::collections::HashSet::new();
    for (a, b) in edges {
        if seen.insert(a.to_string()) {
            node_names.push(a.to_string());
        }
        if seen.insert(b.to_string()) {
            node_names.push(b.to_string());
        }
    }

    (graph, node_map, node_names)
}

fn find_connected_components(
    graph: &Graph<(), (), Undirected>,
    node_map: &HashMap<String, NodeIndex>,
    node_names: &[String],
) -> Vec<Vec<String>> {
    let mut components: Vec<Vec<String>> = Vec::new();
    let mut visited: HashMap<NodeIndex, bool> = HashMap::new();

    for node in graph.node_indices() {
        if visited.contains_key(&node) {
            continue;
        }

        let mut component = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            if visited.contains_key(&n) {
                continue;
            }
            visited.insert(n, true);

            for name in node_names {
                if node_map.get(name.as_str()) == Some(&n) {
                    component.push(name.clone());
                    break;
                }
            }

            for neighbor in graph.neighbors(n) {
                if !visited.contains_key(&neighbor) {
                    stack.push(neighbor);
                }
            }
        }
        component.sort();
        components.push(component);
    }

    components
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::CheckStatus;

    #[test]
    fn test_fully_connected_mesh() {
        let edges = vec![("A", "B"), ("B", "C"), ("C", "A")];
        let results = check_connectivity(&edges);
        let fails: Vec<_> = results
            .iter()
            .filter(|r| r.status == CheckStatus::Fail)
            .collect();
        assert!(fails.is_empty(), "all should pass for mesh: {:?}", fails);
    }

    #[test]
    fn test_disconnected_graph() {
        let edges = vec![("A", "B"), ("C", "D")];
        let results = check_connectivity(&edges);
        let dc = results
            .iter()
            .find(|r| r.name == "disconnected-components")
            .unwrap();
        assert_eq!(dc.status, CheckStatus::Fail);
    }

    #[test]
    fn test_empty_network() {
        let results = check_connectivity(&[]);
        let empty = results.iter().find(|r| r.name == "empty-network").unwrap();
        assert_eq!(empty.status, CheckStatus::Error);
    }
}
