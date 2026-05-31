//! Simulated link between two virtual nodes with configurable quality parameters.

use std::time::Duration;

/// A bidirectional simulated link between two nodes.
///
/// Models real-world link characteristics:
/// - Base latency + random jitter
/// - Packet loss (independent per packet)
/// - Bandwidth cap
/// - Signal quality (SNR)
#[derive(Debug, Clone)]
pub struct VirtualLink {
    /// Unique link identifier.
    #[allow(dead_code)]
    pub id: String,
    /// First endpoint node ID.
    pub node_a: String,
    /// Second endpoint node ID.
    pub node_b: String,
    /// Base one-way latency.
    pub latency: Duration,
    /// Maximum random jitter added to latency (±jitter/2 on each packet).
    pub jitter: Duration,
    /// Packet loss probability (0.0 = none, 1.0 = all packets lost).
    pub packet_loss: f64,
    /// Maximum bandwidth in bytes/sec (0 = unlimited).
    #[allow(dead_code)]
    pub bandwidth: u64,
    #[allow(dead_code)]
    pub snr: f64,
}

impl VirtualLink {
    /// Create a new link with default parameters.
    #[allow(dead_code)]
    pub fn new(id: &str, node_a: &str, node_b: &str) -> Self {
        VirtualLink {
            id: id.to_string(),
            node_a: node_a.to_string(),
            node_b: node_b.to_string(),
            latency: Duration::from_millis(10),
            jitter: Duration::from_millis(2),
            packet_loss: 0.0,
            bandwidth: 0,
            snr: 30.0,
        }
    }

    /// Check whether the other endpoint is connected to this link.
    #[allow(dead_code)]
    pub fn connects(&self, node_id: &str) -> bool {
        self.node_a == node_id || self.node_b == node_id
    }

    /// Given one endpoint, return the other.
    pub fn peer(&self, node_id: &str) -> Option<&str> {
        if self.node_a == node_id {
            Some(&self.node_b)
        } else if self.node_b == node_id {
            Some(&self.node_a)
        } else {
            None
        }
    }

    /// Determine whether a packet is lost on this link.
    pub fn is_packet_lost(&self, rng: &mut impl FnMut() -> f64) -> bool {
        if self.packet_loss <= 0.0 {
            return false;
        }
        rng() < self.packet_loss
    }

    /// Calculate the effective one-way latency including jitter.
    pub fn effective_latency(&self, rng: &mut impl FnMut() -> f64) -> Duration {
        if self.jitter.is_zero() {
            return self.latency;
        }
        // Jitter is ±jitter/2 applied uniformly
        let jitter_ms = self.jitter.as_secs_f64() * 1000.0;
        let offset = (rng() - 0.5) * jitter_ms;
        let total_ms = (self.latency.as_secs_f64() * 1000.0 + offset).max(0.0);
        Duration::from_secs_f64(total_ms / 1000.0)
    }
}

/// Builder for constructing links with quality presets.
#[derive(Debug, Clone, Default)]
pub struct LinkBuilder {
    pub latency_ms: u64,
    pub jitter_ms: u64,
    pub packet_loss: f64,
    pub bandwidth: u64,
    pub snr: f64,
}

impl LinkBuilder {
    /// Excellent link: low latency, no loss, high SNR.
    pub fn excellent() -> Self {
        LinkBuilder {
            latency_ms: 2,
            jitter_ms: 1,
            packet_loss: 0.001,
            bandwidth: 100_000_000, // 100 Mbps
            snr: 40.0,
        }
    }

    /// Good link: typical Wi-Fi or wired LAN.
    pub fn good() -> Self {
        LinkBuilder {
            latency_ms: 10,
            jitter_ms: 3,
            packet_loss: 0.01,
            bandwidth: 10_000_000, // 10 Mbps
            snr: 25.0,
        }
    }

    /// Moderate link: long-range Wi-Fi or cellular.
    pub fn moderate() -> Self {
        LinkBuilder {
            latency_ms: 50,
            jitter_ms: 15,
            packet_loss: 0.05,
            bandwidth: 1_000_000, // 1 Mbps
            snr: 15.0,
        }
    }

    /// Poor link: weak LoRa or congested link.
    pub fn poor() -> Self {
        LinkBuilder {
            latency_ms: 200,
            jitter_ms: 50,
            packet_loss: 0.15,
            bandwidth: 100_000, // 100 Kbps
            snr: 5.0,
        }
    }

    /// Build a link with this builder's parameters.
    pub fn build(&self, id: &str, node_a: &str, node_b: &str) -> VirtualLink {
        VirtualLink {
            id: id.to_string(),
            node_a: node_a.to_string(),
            node_b: node_b.to_string(),
            latency: Duration::from_millis(self.latency_ms),
            jitter: Duration::from_millis(self.jitter_ms),
            packet_loss: self.packet_loss,
            bandwidth: self.bandwidth,
            snr: self.snr,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_peer() {
        let link = VirtualLink::new("l0", "node_a", "node_b");
        assert_eq!(link.peer("node_a"), Some("node_b"));
        assert_eq!(link.peer("node_b"), Some("node_a"));
        assert_eq!(link.peer("node_c"), None);
    }

    #[test]
    fn test_connects() {
        let link = VirtualLink::new("l0", "node_a", "node_b");
        assert!(link.connects("node_a"));
        assert!(link.connects("node_b"));
        assert!(!link.connects("node_c"));
    }

    #[test]
    fn test_packet_loss_never_with_zero() {
        let link = VirtualLink::new("l0", "a", "b");
        let mut calls = 0;
        let lost = link.is_packet_lost(&mut || {
            calls += 1;
            0.5
        });
        assert!(!lost);
        assert_eq!(calls, 0); // short-circuits, doesn't call rng
    }

    #[test]
    fn test_effective_latency_no_jitter() {
        let link = VirtualLink::new("l0", "a", "b");
        let latency = link.effective_latency(&mut || 0.5);
        assert_eq!(latency, Duration::from_millis(10));
    }

    #[test]
    fn test_builder_excellent() {
        let b = LinkBuilder::excellent();
        assert!(b.latency_ms < 10);
        assert!(b.packet_loss < 0.01);
    }
}
