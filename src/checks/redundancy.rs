use crate::checks::{CheckCategory, CheckResult};
use petgraph::graph::{Graph, NodeIndex};
use petgraph::Undirected;
use std::collections::HashMap;

pub fn check_redundancy(topology_edges: &[(&str, &str)]) -> Vec<CheckResult> {
    let mut results = Vec::new();

    if topology_edges.is_empty() {
        results.push(CheckResult::warn(
            CheckCategory::Redundancy,
            "empty-topology",
            "no topology edges defined",
        ));
        return results;
    }

    let (graph, node_map, _) = build_graph(topology_edges);

    if graph.node_count() == 0 {
        results.push(CheckResult::warn(
            CheckCategory::Redundancy,
            "empty-graph",
            "no nodes found in topology",
        ));
        return results;
    }

    let cut_vertices = find_articulation_points(&graph, &node_map);

    if cut_vertices.is_empty() {
        results.push(CheckResult::pass(
            CheckCategory::Redundancy,
            "articulation-points",
            "no single points of failure found",
        ));
    } else {
        results.push(
            CheckResult::warn(
                CheckCategory::Redundancy,
                "articulation-points",
                &format!("found {} articulation point(s)", cut_vertices.len()),
            )
            .with_details(&format!("Articulation points: {}", cut_vertices.join(", "))),
        );
    }

    let bridges = find_bridges(&graph, &node_map);

    if bridges.is_empty() {
        results.push(CheckResult::pass(
            CheckCategory::Redundancy,
            "bridges",
            "no bridge links found",
        ));
    } else {
        results.push(
            CheckResult::warn(
                CheckCategory::Redundancy,
                "bridges",
                &format!("found {} bridge link(s)", bridges.len()),
            )
            .with_details(&format!("Bridges: {}", bridges.join(", "))),
        );
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
    let mut insertion_order: Vec<String> = Vec::new();

    for (a, b) in edges {
        if !node_map.contains_key(*a) {
            let idx = graph.add_node(());
            node_map.insert(a.to_string(), idx);
            insertion_order.push(a.to_string());
        }
        if !node_map.contains_key(*b) {
            let idx = graph.add_node(());
            node_map.insert(b.to_string(), idx);
            insertion_order.push(b.to_string());
        }
        let na = node_map[*a];
        let nb = node_map[*b];
        if !graph.neighbors(na).any(|n| n == nb) {
            graph.add_edge(na, nb, ());
        }
    }

    let mut seen = std::collections::HashSet::new();
    insertion_order.retain(|n| seen.insert(n.clone()));

    (graph, node_map, insertion_order)
}

fn find_articulation_points(
    graph: &Graph<(), (), Undirected>,
    node_map: &HashMap<String, NodeIndex>,
) -> Vec<String> {
    let idx_to_name: HashMap<NodeIndex, String> =
        node_map.iter().map(|(k, v)| (*v, k.clone())).collect();

    if graph.node_count() == 0 {
        return Vec::new();
    }

    let n = graph.node_count();
    let mut visited = vec![false; n];
    let mut disc = vec![0i32; n];
    let mut low = vec![0i32; n];
    let mut parent = vec![-1i32; n];
    let mut ap = vec![false; n];
    let mut time = 0;

    let idx_to_usize: HashMap<NodeIndex, usize> = graph
        .node_indices()
        .enumerate()
        .map(|(i, idx)| (idx, i))
        .collect();
    let usize_to_idx: Vec<NodeIndex> = (0..n)
        .filter_map(|i| graph.node_indices().nth(i))
        .collect();

    #[expect(clippy::too_many_arguments)]
    fn dfs_ap(
        graph: &Graph<(), (), Undirected>,
        u: usize,
        visited: &mut [bool],
        disc: &mut [i32],
        low: &mut [i32],
        parent: &mut [i32],
        ap: &mut [bool],
        time: &mut i32,
        idx_to_usize: &HashMap<NodeIndex, usize>,
        usize_to_idx: &[NodeIndex],
    ) {
        let mut children = 0;
        visited[u] = true;
        *time += 1;
        disc[u] = *time;
        low[u] = *time;

        let u_node = usize_to_idx[u];
        for v_node in graph.neighbors(u_node) {
            let v = idx_to_usize[&v_node];
            if !visited[v] {
                children += 1;
                parent[v] = u as i32;
                dfs_ap(graph, v, visited, disc, low, parent, ap, time, idx_to_usize, usize_to_idx);
                low[u] = low[u].min(low[v]);
                if parent[u] == -1 && children > 1 {
                    ap[u] = true;
                }
                if parent[u] != -1 && low[v] >= disc[u] {
                    ap[u] = true;
                }
            } else if v != parent[u] as usize {
                low[u] = low[u].min(disc[v]);
            }
        }
    }

    for i in 0..n {
        if !visited[i] {
            dfs_ap(
                graph,
                i,
                &mut visited,
                &mut disc,
                &mut low,
                &mut parent,
                &mut ap,
                &mut time,
                &idx_to_usize,
                &usize_to_idx,
            );
        }
    }

    let mut points: Vec<String> = Vec::new();
    for (idx, is_ap) in ap.iter().enumerate() {
        if *is_ap {
            if let Some(node_idx) = graph.node_indices().nth(idx) {
                if let Some(name) = idx_to_name.get(&node_idx) {
                    points.push(name.clone());
                }
            }
        }
    }
    points.sort();
    points
}

fn find_bridges(
    graph: &Graph<(), (), Undirected>,
    node_map: &HashMap<String, NodeIndex>,
) -> Vec<String> {
    let idx_to_name: HashMap<NodeIndex, String> =
        node_map.iter().map(|(k, v)| (*v, k.clone())).collect();
    let mut bridges: Vec<(String, String)> = Vec::new();

    if graph.node_count() == 0 {
        return Vec::new();
    }

    let n = graph.node_count();
    let mut visited = vec![false; n];
    let mut disc = vec![0i32; n];
    let mut low = vec![0i32; n];
    let mut parent = vec![-1i32; n];
    let mut time = 0;

    let e_idx_to_usize: HashMap<NodeIndex, usize> = graph
        .node_indices()
        .enumerate()
        .map(|(i, idx)| (idx, i))
        .collect();
    let e_usize_to_idx: Vec<NodeIndex> = (0..n)
        .filter_map(|i| graph.node_indices().nth(i))
        .collect();

    #[expect(clippy::too_many_arguments)]
    fn dfs_bridges(
        graph: &Graph<(), (), Undirected>,
        u: usize,
        visited: &mut [bool],
        disc: &mut [i32],
        low: &mut [i32],
        parent: &mut [i32],
        time: &mut i32,
        bridges: &mut Vec<(String, String)>,
        idx_to_name: &HashMap<NodeIndex, String>,
        e_idx_to_usize: &HashMap<NodeIndex, usize>,
        e_usize_to_idx: &[NodeIndex],
    ) {
        visited[u] = true;
        *time += 1;
        disc[u] = *time;
        low[u] = *time;

        let u_node = e_usize_to_idx[u];

        for v_node in graph.neighbors(u_node) {
            let v = e_idx_to_usize[&v_node];
            if !visited[v] {
                parent[v] = u as i32;
                dfs_bridges(
                    graph,
                    v,
                    visited,
                    disc,
                    low,
                    parent,
                    time,
                    bridges,
                    idx_to_name,
                    e_idx_to_usize,
                    e_usize_to_idx,
                );
                low[u] = low[u].min(low[v]);
                if low[v] > disc[u] {
                    let u_name = idx_to_name
                        .get(&u_node)
                        .cloned()
                        .unwrap_or_else(|| "?".to_string());
                    let v_name = idx_to_name
                        .get(&v_node)
                        .cloned()
                        .unwrap_or_else(|| "?".to_string());
                    bridges.push((u_name, v_name));
                }
            } else if v != parent[u] as usize {
                low[u] = low[u].min(disc[v]);
            }
        }
    }

    for i in 0..n {
        if !visited[i] {
            dfs_bridges(
                graph,
                i,
                &mut visited,
                &mut disc,
                &mut low,
                &mut parent,
                &mut time,
                &mut bridges,
                &idx_to_name,
                &e_idx_to_usize,
                &e_usize_to_idx,
            );
        }
    }

    bridges.sort();
    bridges
        .iter()
        .map(|(a, b)| format!("{} -- {}", a, b))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::CheckStatus;

    #[test]
    fn test_fully_connected_mesh_no_cut_vertices() {
        let edges = vec![("A", "B"), ("B", "C"), ("C", "A")];
        let results = check_redundancy(&edges);
        let ap = results
            .iter()
            .find(|r| r.name == "articulation-points")
            .unwrap();
        assert_eq!(ap.status, CheckStatus::Pass);
    }

    #[test]
    fn test_chain_has_articulation_points() {
        let edges = vec![("A", "B"), ("B", "C"), ("C", "D")];
        let results = check_redundancy(&edges);
        let ap = results
            .iter()
            .find(|r| r.name == "articulation-points")
            .unwrap();
        assert_eq!(ap.status, CheckStatus::Warning);
    }

    #[test]
    fn test_star_has_articulation_points() {
        let edges = vec![("center", "A"), ("center", "B"), ("center", "C")];
        let results = check_redundancy(&edges);
        let ap = results
            .iter()
            .find(|r| r.name == "articulation-points")
            .unwrap();
        assert_eq!(ap.status, CheckStatus::Warning);
    }

    #[test]
    fn test_chain_has_bridges() {
        let edges = vec![("A", "B"), ("B", "C")];
        let results = check_redundancy(&edges);
        let bridges = results.iter().find(|r| r.name == "bridges").unwrap();
        assert_eq!(bridges.status, CheckStatus::Warning);
    }

    #[test]
    fn test_empty_edges() {
        let results = check_redundancy(&[]);
        assert!(results.iter().any(|r| r.name == "empty-topology"));
    }
}
