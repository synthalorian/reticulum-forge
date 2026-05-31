//! Simulated Reticulum node — identity, routing table, and announce handling.

use std::collections::HashMap;
use std::time::Duration;

/// A short unique identifier for a virtual node.
pub type NodeId = String;

/// A simulated packet traversing the network.
#[derive(Debug, Clone)]
pub struct Packet {
    /// Unique packet ID for tracing.
    #[allow(dead_code)]
    pub id: u64,
    /// Sender node ID.
    pub src: NodeId,
    /// Destination node ID (empty = broadcast/announce).
    pub dst: NodeId,
    /// Payload label for identification in metrics.
    #[allow(dead_code)]
    pub payload: String,
    /// Number of hops so far.
    pub hops: u32,
    /// Total accumulated latency from traversed links.
    pub accumulated_latency: Duration,
    /// Whether this is an announce/broadcast packet.
    pub is_announce: bool,
}

impl Packet {
    /// Create a new data packet.
    pub fn new(id: u64, src: &str, dst: &str, payload: &str) -> Self {
        Packet {
            id,
            src: src.to_string(),
            dst: dst.to_string(),
            payload: payload.to_string(),
            hops: 0,
            accumulated_latency: Duration::ZERO,
            is_announce: false,
        }
    }

    /// Create a new announce (broadcast) packet.
    pub fn announce(id: u64, src: &str, payload: &str) -> Self {
        Packet {
            id,
            src: src.to_string(),
            dst: String::new(),
            payload: payload.to_string(),
            hops: 0,
            accumulated_latency: Duration::ZERO,
            is_announce: true,
        }
    }
}

/// A simulated Reticulum node.
///
/// Nodes have an identity, maintain a routing table, and can send/receive packets.
/// In simulation, nodes don't actually run Reticulum — they model the protocol's
/// announce-propagation and path-discovery behavior at a high level.
#[derive(Debug, Clone)]
pub struct VirtualNode {
    pub id: NodeId,
    /// Routing table: destination -> next hop node ID.
    pub routing_table: HashMap<NodeId, NodeId>,
    /// Path costs: destination -> hop count.
    pub path_costs: HashMap<NodeId, u32>,
    /// Sequence number for unique announce identification.
    #[allow(dead_code)]
    announce_seq: u64,
}

impl VirtualNode {
    pub fn new(id: &str) -> Self {
        VirtualNode {
            id: id.to_string(),
            routing_table: HashMap::new(),
            path_costs: HashMap::new(),
            announce_seq: 0,
        }
    }

    /// Process an announce packet received from a neighbor.
    /// Returns `true` if this is *new* information (the announce propagated further).
    pub fn process_announce(&mut self, from: &str, packet: &Packet) -> bool {
        let cost = packet.hops + 1;
        let better_path = match self.path_costs.get(&packet.src) {
            Some(&existing) => cost < existing,
            None => true, // first time hearing about this source
        };

        if better_path {
            self.routing_table
                .insert(packet.src.clone(), from.to_string());
            self.path_costs.insert(packet.src.clone(), cost);
            true
        } else {
            false
        }
    }

    /// Generate an announce packet from this node.
    #[allow(dead_code)]
    pub fn generate_announce(&mut self, packet_id: &mut u64) -> Packet {
        self.announce_seq += 1;
        let id = *packet_id;
        *packet_id += 1;
        let mut p = Packet::announce(id, &self.id, &format!("announce-{}", self.announce_seq));
        p.hops = 0;
        p
    }

    /// Generate a data packet destined for a specific node.
    pub fn generate_packet(&self, packet_id: &mut u64, dst: &str, payload: &str) -> Option<Packet> {
        if !self.routing_table.contains_key(dst) {
            return None; // no route
        }
        let id = *packet_id;
        *packet_id += 1;
        Some(Packet::new(id, &self.id, dst, payload))
    }

    /// Get the next hop for a destination, if known.
    pub fn next_hop(&self, dst: &str) -> Option<&NodeId> {
        self.routing_table.get(dst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = VirtualNode::new("node_1");
        assert_eq!(node.id, "node_1");
        assert!(node.routing_table.is_empty());
    }

    #[test]
    fn test_announce_creates_route() {
        let mut node = VirtualNode::new("node_a");
        let mut pkt_id = 0;
        let mut sender = VirtualNode::new("node_b");
        let announce = sender.generate_announce(&mut pkt_id);

        let propagated = node.process_announce("node_c", &announce);
        assert!(propagated);
        assert_eq!(
            node.routing_table.get("node_b"),
            Some(&"node_c".to_string())
        );
        assert_eq!(node.path_costs.get("node_b"), Some(&1));
    }

    #[test]
    fn test_better_path_wins() {
        let mut node = VirtualNode::new("node_a");
        let mut pkt_id = 0;
        let mut sender = VirtualNode::new("node_b");

        // First announce via node_c (cost 1)
        let mut a1 = sender.generate_announce(&mut pkt_id);
        a1.hops = 0;
        node.process_announce("node_c", &a1);
        assert_eq!(node.path_costs.get("node_b"), Some(&1));

        // Second announce via node_d (cost 3) — worse path, should be ignored
        let mut a2 = sender.generate_announce(&mut pkt_id);
        a2.hops = 2;
        node.process_announce("node_d", &a2);
        assert_eq!(node.path_costs.get("node_b"), Some(&1)); // still 1
    }

    #[test]
    fn test_generate_packet_requires_route() {
        let node = VirtualNode::new("node_a");
        let mut pkt_id = 0;
        assert!(node
            .generate_packet(&mut pkt_id, "unknown", "hello")
            .is_none());
    }
}
