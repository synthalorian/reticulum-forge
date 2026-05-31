//! Metrics collection for the network simulator.
//!
//! Tracks delivery rates, latency distributions, and hop counts across
//! all packets sent during a simulation run.

use crate::simulate::node::Packet;
use serde::Serialize;

/// Aggregated metrics from a simulation run.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Metrics {
    // Packet counts
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_delivered: u64,
    pub packets_lost: u64,
    pub announces_sent: u64,
    pub announces_propagated: u64,

    // Latency (accumulated for delivered packets)
    pub total_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub latency_samples: Vec<f64>,

    // Hop counts
    pub total_hops: u64,
    pub min_hops: u32,
    pub max_hops: u32,
    pub hop_samples: Vec<u32>,

    // Path discovery
    pub routes_discovered: u64,
}

impl Metrics {
    /// Record a packet being sent.
    pub fn record_sent(&mut self, packet: &Packet) {
        if packet.is_announce {
            self.announces_sent += 1;
        } else {
            self.packets_sent += 1;
        }
    }

    /// Record a packet being received (not yet delivered to final dst).
    pub fn record_received(&mut self, _packet: &Packet) {
        self.packets_received += 1;
    }

    /// Record a packet being delivered to its final destination.
    pub fn record_delivered(&mut self, packet: &Packet) {
        self.packets_delivered += 1;

        let lat_ms = packet.accumulated_latency.as_secs_f64() * 1000.0;
        self.total_latency_ms += lat_ms;
        self.latency_samples.push(lat_ms);

        if lat_ms < self.min_latency_ms || self.min_latency_ms == 0.0 {
            self.min_latency_ms = lat_ms;
        }
        if lat_ms > self.max_latency_ms {
            self.max_latency_ms = lat_ms;
        }

        self.total_hops += packet.hops as u64;
        self.hop_samples.push(packet.hops);

        if packet.hops < self.min_hops || self.min_hops == 0 {
            self.min_hops = packet.hops;
        }
        if packet.hops > self.max_hops {
            self.max_hops = packet.hops;
        }
    }

    /// Record a packet that was lost.
    pub fn record_lost(&mut self) {
        self.packets_lost += 1;
    }

    /// Record an announce propagation event.
    pub fn record_announce_propagation(&mut self) {
        self.announces_propagated += 1;
    }

    /// Record a new route being discovered.
    pub fn record_route_discovered(&mut self) {
        self.routes_discovered += 1;
    }

    // ---- derived metrics ----

    /// Delivery rate as a fraction (0.0 – 1.0).
    pub fn delivery_rate(&self) -> f64 {
        let total_sent = self.packets_sent + self.announces_sent;
        if total_sent == 0 {
            return 0.0;
        }
        self.packets_delivered as f64 / total_sent as f64
    }

    /// Average latency of delivered packets in milliseconds.
    pub fn avg_latency_ms(&self) -> f64 {
        if self.packets_delivered == 0 {
            return 0.0;
        }
        self.total_latency_ms / self.packets_delivered as f64
    }

    /// Average hop count of delivered packets.
    pub fn avg_hops(&self) -> f64 {
        if self.packets_delivered == 0 {
            return 0.0;
        }
        self.total_hops as f64 / self.packets_delivered as f64
    }

    /// Packet loss rate as a fraction (0.0 – 1.0) for data packets only.
    pub fn loss_rate(&self) -> f64 {
        let total_data = self.packets_sent;
        if total_data == 0 {
            return 0.0;
        }
        self.packets_lost as f64 / total_data as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulate::node::Packet;
    use std::time::Duration;

    #[test]
    fn test_empty_metrics() {
        let m = Metrics::default();
        assert_eq!(m.delivery_rate(), 0.0);
        assert_eq!(m.avg_latency_ms(), 0.0);
    }

    #[test]
    fn test_record_delivered_updates_stats() {
        let mut m = Metrics::default();
        let mut p = Packet::new(1, "a", "b", "test");
        p.hops = 3;
        p.accumulated_latency = Duration::from_millis(150);

        m.record_delivered(&p);
        assert_eq!(m.packets_delivered, 1);
        assert_eq!(m.min_hops, 3);
        assert_eq!(m.max_hops, 3);
        assert!((m.avg_latency_ms() - 150.0).abs() < 0.001);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_delivery_rate() {
        let mut m = Metrics::default();
        m.packets_sent = 10;
        m.packets_delivered = 7;
        assert!((m.delivery_rate() - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_multiple_latency_samples() {
        let mut m = Metrics::default();
        for i in 0..3 {
            let mut p = Packet::new(i, "a", "b", "t");
            p.accumulated_latency = Duration::from_millis(100 * (i + 1));
            m.record_delivered(&p);
        }
        assert_eq!(m.latency_samples.len(), 3);
        assert!((m.avg_latency_ms() - 200.0).abs() < 0.001);
        assert!((m.min_latency_ms - 100.0).abs() < 0.001);
        assert!((m.max_latency_ms - 300.0).abs() < 0.001);
    }
}
