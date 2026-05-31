//! Event-driven simulation engine for virtual Reticulum networks.
//!
//! The engine runs a two-phase simulation:
//! 1. **Announce propagation** — flood announces through the topology so every
//!    node learns routes to every other node.
//! 2. **Data phase** — for each tick, random node pairs exchange data packets
//!    through the discovered routes. Links apply latency, jitter, and packet loss.

use crate::simulate::link::VirtualLink;
use crate::simulate::metrics::Metrics;
use crate::simulate::node::{Packet, VirtualNode};
use crate::simulate::report::SimulationReport;
use crate::simulate::topology::Topology;
use rand::Rng;
use std::collections::HashMap;
use std::time::Duration;

/// Simulation configuration.
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// Total simulated duration.
    pub duration: Duration,
    /// How often (in sim time) nodes send data packets.
    pub data_interval: Duration,
    /// How often (in sim time) nodes re-announce.
    #[allow(dead_code)]
    pub announce_interval: Duration,
    /// How many data packets to attempt per interval per node pair.
    pub packets_per_cycle: usize,
}

impl Default for SimConfig {
    fn default() -> Self {
        SimConfig {
            duration: Duration::from_secs(60),
            data_interval: Duration::from_secs(5),
            announce_interval: Duration::from_secs(30),
            packets_per_cycle: 3,
        }
    }
}

/// The simulation engine.
pub struct SimEngine {
    nodes: HashMap<String, VirtualNode>,
    links: Vec<VirtualLink>,
    metrics: Metrics,
    packet_id_counter: u64,
    config: SimConfig,
    /// Adjacency list: node_id -> Vec of link indices
    adj: HashMap<String, Vec<usize>>,
}

impl SimEngine {
    /// Create a new simulation engine from a topology.
    pub fn new(topology: Topology, config: SimConfig) -> Self {
        let nodes: HashMap<String, VirtualNode> = topology
            .nodes
            .into_iter()
            .map(|n| (n.id.clone(), n))
            .collect();

        // Build adjacency list
        let mut adj: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, link) in topology.links.iter().enumerate() {
            adj.entry(link.node_a.clone()).or_default().push(idx);
            adj.entry(link.node_b.clone()).or_default().push(idx);
        }

        SimEngine {
            nodes,
            links: topology.links,
            metrics: Metrics::default(),
            packet_id_counter: 0,
            config,
            adj,
        }
    }

    /// Run the full simulation and return a report.
    pub async fn run(&mut self) -> SimulationReport {
        // Phase 1: flood announces to build routing tables
        self.propagate_announces();

        // Phase 2: simulate data traffic
        self.simulate_data_traffic();

        SimulationReport {
            duration: self.config.duration,
            metrics: self.metrics.clone(),
            node_count: self.nodes.len(),
            link_count: self.links.len(),
        }
    }

    /// Phase 1: flood announces from every node to build routing tables.
    ///
    /// This is essentially BFS from each node through the topology.
    fn propagate_announces(&mut self) {
        let node_ids: Vec<String> = self.nodes.keys().cloned().collect();

        for src_id in &node_ids {
            let mut frontier = vec![src_id.clone()];
            let mut visited = std::collections::HashSet::new();
            visited.insert(src_id.clone());

            let mut hop = 0u32;

            while !frontier.is_empty() {
                let mut next_frontier = Vec::new();

                for current in &frontier {
                    // Get neighbors via adjacency
                    let neighbor_ids = self.neighbor_ids(current);
                    let current_id = current.clone();

                    for neighbor_id in &neighbor_ids {
                        if visited.contains(neighbor_id) {
                            continue;
                        }

                        // Simulate announce packet
                        let mut announce = Packet::announce(
                            self.next_packet_id(),
                            src_id,
                            &format!("announce-{}-hop{}", src_id, hop + 1),
                        );
                        announce.hops = hop;

                        // Check packet loss on the link between current and neighbor
                        let link_idx = self.link_between(&current_id, neighbor_id);
                        let link = &self.links[link_idx];
                        let mut rng = rand::thread_rng();
                        if link.is_packet_lost(&mut || rng.gen()) {
                            self.metrics.record_lost();
                            continue; // announce lost on this link
                        }

                        // Forward announce to neighbor
                        if let Some(neighbor_node) = self.nodes.get_mut(neighbor_id) {
                            let propagated = neighbor_node.process_announce(&current_id, &announce);
                            if propagated {
                                self.metrics.record_announce_propagation();
                                self.metrics.record_route_discovered();
                            }
                        }

                        visited.insert(neighbor_id.clone());
                        next_frontier.push(neighbor_id.clone());
                    }
                }

                frontier = next_frontier;
                hop += 1;
            }
        }
    }

    /// Phase 2: simulate data traffic between random node pairs.
    fn simulate_data_traffic(&mut self) {
        let node_ids: Vec<String> = self.nodes.keys().cloned().collect();
        let n = node_ids.len();
        if n < 2 {
            return;
        }

        let total_ticks = self
            .config
            .duration
            .as_secs()
            .max(1)
            .div_ceil(self.config.data_interval.as_secs().max(1));

        let mut rng = rand::thread_rng();

        for _tick in 0..total_ticks {
            // Pick random node pairs to send data
            for _ in 0..self.config.packets_per_cycle {
                let src_idx = rng.gen_range(0..n);
                let mut dst_idx = rng.gen_range(0..n);
                while dst_idx == src_idx && n > 1 {
                    dst_idx = rng.gen_range(0..n);
                }

                let src_id = &node_ids[src_idx];
                let dst_id = &node_ids[dst_idx];

                // Generate packet from src
                let packet = {
                    let src_node = self.nodes.get(src_id).unwrap();
                    match src_node.generate_packet(&mut self.packet_id_counter, dst_id, "data") {
                        Some(p) => p,
                        None => continue, // no route to dst
                    }
                };

                self.metrics.record_sent(&packet);

                // Route the packet through the network
                let _ = self.route_packet(packet);
            }
        }
    }

    /// Route a single packet from source to destination through routing tables.
    ///
    /// Returns the number of hops taken, or None if the packet was lost.
    fn route_packet(&mut self, mut packet: Packet) -> Option<u32> {
        let mut current = packet.src.clone();
        let dst = packet.dst.clone();
        let mut rng = rand::thread_rng();

        loop {
            // Find next hop
            let next_hop = {
                let node = self.nodes.get(&current)?;
                node.next_hop(&dst).cloned()?
            };

            // Find the link between current and next_hop
            let link_idx = self.link_between(&current, &next_hop);
            let link = &self.links[link_idx];

            // Check packet loss
            if link.is_packet_lost(&mut || rng.gen()) {
                self.metrics.record_lost();
                return None;
            }

            // Apply latency
            let latency = link.effective_latency(&mut || rng.gen());
            packet.accumulated_latency += latency;
            packet.hops += 1;

            // Record at intermediate hop
            self.metrics.record_received(&packet);

            // Check if we've arrived
            if next_hop == dst {
                self.metrics.record_delivered(&packet);
                return Some(packet.hops);
            }

            current = next_hop;
        }
    }

    // ---- helpers ----

    fn next_packet_id(&mut self) -> u64 {
        let id = self.packet_id_counter;
        self.packet_id_counter += 1;
        id
    }

    fn neighbor_ids(&self, node_id: &str) -> Vec<String> {
        let mut neighbors = Vec::new();
        if let Some(link_indices) = self.adj.get(node_id) {
            for &idx in link_indices {
                if let Some(peer) = self.links[idx].peer(node_id) {
                    neighbors.push(peer.to_string());
                }
            }
        }
        neighbors
    }

    fn link_between(&self, a: &str, b: &str) -> usize {
        for (idx, link) in self.links.iter().enumerate() {
            if (link.node_a == a && link.node_b == b) || (link.node_a == b && link.node_b == a) {
                return idx;
            }
        }
        panic!("no link between {} and {}", a, b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulate::topology::{generate, TopologyConfig, TopologyType};

    fn small_mesh() -> (Topology, SimConfig) {
        let config = TopologyConfig {
            node_count: 5,
            topology_type: TopologyType::Mesh,
            link_quality: "excellent".to_string(),
            partial_degree: Some(4),
        };
        let topo = generate(&config);
        let sim_config = SimConfig {
            duration: Duration::from_secs(10),
            data_interval: Duration::from_secs(5),
            ..Default::default()
        };
        (topo, sim_config)
    }

    #[tokio::test]
    async fn test_small_mesh_simulates() {
        let (topo, sim_config) = small_mesh();
        let mut engine = SimEngine::new(topo, sim_config);
        let report = engine.run().await;
        assert_eq!(report.node_count, 5);
        // In a fully-connected 5-node mesh with excellent links,
        // all routes should be discovered
        assert!(report.metrics.routes_discovered > 0);
    }

    #[tokio::test]
    async fn test_chain_vs_mesh_metrics() {
        // Create a chain topology
        let chain_config = TopologyConfig {
            node_count: 10,
            topology_type: TopologyType::Chain,
            link_quality: "excellent".to_string(),
            partial_degree: None,
        };
        let chain_topo = generate(&chain_config);
        let mut chain_engine = SimEngine::new(
            chain_topo,
            SimConfig {
                duration: Duration::from_secs(10),
                ..Default::default()
            },
        );
        let chain_report = chain_engine.run().await;

        // Create a mesh topology
        let mesh_config = TopologyConfig {
            node_count: 10,
            topology_type: TopologyType::Mesh,
            link_quality: "excellent".to_string(),
            partial_degree: Some(9),
        };
        let mesh_topo = generate(&mesh_config);
        let mut mesh_engine = SimEngine::new(
            mesh_topo,
            SimConfig {
                duration: Duration::from_secs(10),
                ..Default::default()
            },
        );
        let mesh_report = mesh_engine.run().await;

        // Mesh should have fewer hops on average than chain
        assert!(mesh_report.metrics.avg_hops() <= chain_report.metrics.avg_hops() + 0.5);
    }
}
