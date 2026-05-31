//! Topology generators — create node+link sets for common network shapes.

use crate::simulate::link::{LinkBuilder, VirtualLink};
use crate::simulate::node::VirtualNode;
use rand::seq::SliceRandom;
use rand::thread_rng;

/// Supported topology types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyType {
    Mesh,
    Star,
    Ring,
    Chain,
}

impl TopologyType {
    /// Parse from a CLI string (case-insensitive).
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mesh" => Some(TopologyType::Mesh),
            "star" => Some(TopologyType::Star),
            "ring" => Some(TopologyType::Ring),
            "chain" => Some(TopologyType::Chain),
            _ => None,
        }
    }

    pub fn all_names() -> &'static [&'static str] {
        &["mesh", "star", "ring", "chain"]
    }
}

/// A fully generated topology with nodes and links.
#[derive(Debug, Clone)]
pub struct Topology {
    pub nodes: Vec<VirtualNode>,
    pub links: Vec<VirtualLink>,
}

/// Build configuration for topology generation.
#[derive(Debug, Clone)]
pub struct TopologyConfig {
    pub node_count: usize,
    pub topology_type: TopologyType,
    /// Link quality preset: "excellent", "good", "moderate", "poor".
    pub link_quality: String,
    /// Degree for partial mesh (mesh only). Default: node_count (full mesh).
    pub partial_degree: Option<usize>,
}

impl Default for TopologyConfig {
    fn default() -> Self {
        TopologyConfig {
            node_count: 10,
            topology_type: TopologyType::Mesh,
            link_quality: "good".to_string(),
            partial_degree: None,
        }
    }
}

/// Generate a topology according to the given config.
pub fn generate(config: &TopologyConfig) -> Topology {
    let builder = link_builder_for_quality(&config.link_quality);
    let mut link_id = 0;

    let nodes: Vec<VirtualNode> = (0..config.node_count)
        .map(|i| VirtualNode::new(&format!("node_{}", i)))
        .collect();

    let links = match config.topology_type {
        TopologyType::Mesh => generate_mesh(&nodes, &builder, &mut link_id, config.partial_degree),
        TopologyType::Star => generate_star(&nodes, &builder, &mut link_id),
        TopologyType::Ring => generate_ring(&nodes, &builder, &mut link_id),
        TopologyType::Chain => generate_chain(&nodes, &builder, &mut link_id),
    };

    Topology { nodes, links }
}

fn link_builder_for_quality(quality: &str) -> LinkBuilder {
    match quality.to_lowercase().as_str() {
        "excellent" => LinkBuilder::excellent(),
        "good" => LinkBuilder::good(),
        "moderate" => LinkBuilder::moderate(),
        "poor" => LinkBuilder::poor(),
        _ => LinkBuilder::good(),
    }
}

fn next_link_id(counter: &mut u64, a: &str, b: &str) -> String {
    let id = *counter;
    *counter += 1;
    format!("link_{}__{}__{}", id, strip_prefix(a), strip_prefix(b))
}

fn strip_prefix(s: &str) -> &str {
    s.trim_start_matches("node_")
}

fn generate_mesh(
    nodes: &[VirtualNode],
    builder: &LinkBuilder,
    link_id: &mut u64,
    degree: Option<usize>,
) -> Vec<VirtualLink> {
    let n = nodes.len();
    if n < 2 {
        return vec![];
    }

    let k = degree.unwrap_or(n - 1).min(n - 1);
    let mut links = Vec::new();

    for i in 0..n {
        // Pick k random neighbors (not including self)
        let mut candidates: Vec<usize> = (0..n).filter(|&j| j != i).collect();
        candidates.shuffle(&mut thread_rng());
        let neighbors: Vec<usize> = candidates.into_iter().take(k).collect();

        for &j in &neighbors {
            if i < j {
                // Only create each link once
                links.push(builder.build(
                    &next_link_id(link_id, &nodes[i].id, &nodes[j].id),
                    &nodes[i].id,
                    &nodes[j].id,
                ));
            }
        }
    }

    links
}

fn generate_star(
    nodes: &[VirtualNode],
    builder: &LinkBuilder,
    link_id: &mut u64,
) -> Vec<VirtualLink> {
    let n = nodes.len();
    if n < 2 {
        return vec![];
    }

    // Node 0 is the hub
    let hub = &nodes[0];
    (1..n)
        .map(|i| {
            builder.build(
                &next_link_id(link_id, &hub.id, &nodes[i].id),
                &hub.id,
                &nodes[i].id,
            )
        })
        .collect()
}

fn generate_ring(
    nodes: &[VirtualNode],
    builder: &LinkBuilder,
    link_id: &mut u64,
) -> Vec<VirtualLink> {
    let n = nodes.len();
    if n < 2 {
        return vec![];
    }

    let mut links = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        if i < j || (i == n - 1 && j == 0) {
            links.push(builder.build(
                &next_link_id(link_id, &nodes[i].id, &nodes[j].id),
                &nodes[i].id,
                &nodes[j].id,
            ));
        }
    }
    links
}

fn generate_chain(
    nodes: &[VirtualNode],
    builder: &LinkBuilder,
    link_id: &mut u64,
) -> Vec<VirtualLink> {
    let n = nodes.len();
    if n < 2 {
        return vec![];
    }

    (0..n - 1)
        .map(|i| {
            builder.build(
                &next_link_id(link_id, &nodes[i].id, &nodes[i + 1].id),
                &nodes[i].id,
                &nodes[i + 1].id,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mesh_fully_connected() {
        let config = TopologyConfig {
            node_count: 5,
            topology_type: TopologyType::Mesh,
            link_quality: "good".to_string(),
            partial_degree: None,
        };
        let topo = generate(&config);
        assert_eq!(topo.nodes.len(), 5);
        // Each of the 5 nodes connected to all other 4 = (5*4)/2 = 10 links
        assert_eq!(topo.links.len(), 10);
    }

    #[test]
    fn test_generate_star() {
        let config = TopologyConfig {
            node_count: 5,
            topology_type: TopologyType::Star,
            link_quality: "good".to_string(),
            partial_degree: None,
        };
        let topo = generate(&config);
        assert_eq!(topo.nodes.len(), 5);
        assert_eq!(topo.links.len(), 4); // 4 leaves
                                         // All links should include node_0
        assert!(topo.links.iter().all(|l| l.connects("node_0")));
    }

    #[test]
    fn test_generate_ring() {
        let config = TopologyConfig {
            node_count: 5,
            topology_type: TopologyType::Ring,
            link_quality: "good".to_string(),
            partial_degree: None,
        };
        let topo = generate(&config);
        assert_eq!(topo.nodes.len(), 5);
        assert_eq!(topo.links.len(), 5);
    }

    #[test]
    fn test_generate_chain() {
        let config = TopologyConfig {
            node_count: 4,
            topology_type: TopologyType::Chain,
            link_quality: "good".to_string(),
            partial_degree: None,
        };
        let topo = generate(&config);
        assert_eq!(topo.nodes.len(), 4);
        assert_eq!(topo.links.len(), 3);
    }

    #[test]
    fn test_topology_type_parsing() {
        assert_eq!(TopologyType::from_str("mesh"), Some(TopologyType::Mesh));
        assert_eq!(TopologyType::from_str("STAR"), Some(TopologyType::Star));
        assert_eq!(TopologyType::from_str("Ring"), Some(TopologyType::Ring));
        assert_eq!(TopologyType::from_str("chain"), Some(TopologyType::Chain));
        assert_eq!(TopologyType::from_str("custom"), None);
    }
}
