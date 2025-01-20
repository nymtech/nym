// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_node_metrics::mixnet::{EgressMixingStats, IngressMixingStats, MixingStats};
use nym_node_metrics::wireguard::WireguardStats;
use nym_node_metrics::NymNodeMetrics;
use time::OffsetDateTime;

// used to calculate traffic rates
#[derive(Debug)]
pub(crate) struct AtLastUpdate {
    time: OffsetDateTime,

    mixnet: LastMixnet,
    wireguard: LastWireguard,
}

impl AtLastUpdate {
    pub(crate) fn is_initial(&self) -> bool {
        self.time == OffsetDateTime::UNIX_EPOCH
    }

    pub(crate) fn rates(&self, previous: &Self) -> RateSinceUpdate {
        let delta_secs = (self.time - previous.time).as_seconds_f64();

        RateSinceUpdate {
            mixnet: self.mixnet.rates(&previous.mixnet, delta_secs),
            wireguard: self.wireguard.rates(&previous.wireguard, delta_secs),
        }
    }
}

impl Default for AtLastUpdate {
    fn default() -> Self {
        AtLastUpdate {
            time: OffsetDateTime::now_utc(),
            mixnet: Default::default(),
            wireguard: Default::default(),
        }
    }
}

impl From<&NymNodeMetrics> for AtLastUpdate {
    fn from(metrics: &NymNodeMetrics) -> Self {
        AtLastUpdate {
            time: OffsetDateTime::now_utc(),
            mixnet: (&metrics.mixnet).into(),
            wireguard: (&metrics.wireguard).into(),
        }
    }
}

#[derive(Debug, Default)]
struct LastMixnet {
    ingres: LastMixnetIngress,
    egress: LastMixnetEgress,
}

impl LastMixnet {
    fn rates(&self, previous: &Self, time_delta_secs: f64) -> MixnetRateSinceUpdate {
        MixnetRateSinceUpdate {
            ingress: self.ingres.rates(&previous.ingres, time_delta_secs),
            egress: self.egress.rates(&previous.egress, time_delta_secs),
        }
    }
}

impl From<&MixingStats> for LastMixnet {
    fn from(value: &MixingStats) -> Self {
        LastMixnet {
            ingres: (&value.ingress).into(),
            egress: (&value.egress).into(),
        }
    }
}

#[derive(Debug, Default)]
struct LastMixnetIngress {
    forward_hop_packets_received: usize,
    final_hop_packets_received: usize,
    malformed_packets_received: usize,
    excessive_delay_packets: usize,
    forward_hop_packets_dropped: usize,
    final_hop_packets_dropped: usize,
}

impl LastMixnetIngress {
    fn rates(&self, previous: &Self, time_delta_secs: f64) -> MixnetIngressRateSinceUpdate {
        let forward_hop_packets_received_delta =
            self.forward_hop_packets_received - previous.forward_hop_packets_received;
        let final_hop_packets_received_delta =
            self.final_hop_packets_received - previous.final_hop_packets_received;
        let malformed_packets_received_delta =
            self.malformed_packets_received - previous.malformed_packets_received;
        let excessive_delay_packets_delta =
            self.excessive_delay_packets - previous.excessive_delay_packets;
        let forward_hop_packets_dropped_delta =
            self.forward_hop_packets_dropped - previous.forward_hop_packets_dropped;
        let final_hop_packets_dropped_delta =
            self.final_hop_packets_dropped - previous.final_hop_packets_dropped;

        MixnetIngressRateSinceUpdate {
            forward_hop_packets_received_sec: forward_hop_packets_received_delta as f64
                / time_delta_secs,
            final_hop_packets_received_sec: final_hop_packets_received_delta as f64
                / time_delta_secs,
            malformed_packets_received_sec: malformed_packets_received_delta as f64
                / time_delta_secs,
            excessive_delay_packets_sec: excessive_delay_packets_delta as f64 / time_delta_secs,
            forward_hop_packets_dropped_sec: forward_hop_packets_dropped_delta as f64
                / time_delta_secs,
            final_hop_packets_dropped_sec: final_hop_packets_dropped_delta as f64 / time_delta_secs,
        }
    }
}

impl From<&IngressMixingStats> for LastMixnetIngress {
    fn from(value: &IngressMixingStats) -> Self {
        LastMixnetIngress {
            forward_hop_packets_received: value.forward_hop_packets_received(),
            final_hop_packets_received: value.final_hop_packets_received(),
            malformed_packets_received: value.malformed_packets_received(),
            excessive_delay_packets: value.excessive_delay_packets(),
            forward_hop_packets_dropped: value.forward_hop_packets_dropped(),
            final_hop_packets_dropped: value.final_hop_packets_dropped(),
        }
    }
}

#[derive(Debug, Default)]
struct LastMixnetEgress {
    forward_hop_packets_sent: usize,
    ack_packets_sent: usize,
    forward_hop_packets_dropped: usize,
}

impl LastMixnetEgress {
    fn rates(&self, previous: &Self, time_delta_secs: f64) -> MixnetEgressRateSinceUpdate {
        let forward_hop_packets_sent_delta =
            self.forward_hop_packets_sent - previous.forward_hop_packets_sent;
        let ack_packets_sent_delta = self.ack_packets_sent - previous.ack_packets_sent;
        let forward_hop_packets_dropped_delta =
            self.forward_hop_packets_dropped - previous.forward_hop_packets_dropped;

        MixnetEgressRateSinceUpdate {
            forward_hop_packets_sent_sec: forward_hop_packets_sent_delta as f64 / time_delta_secs,
            ack_packets_sent_sec: ack_packets_sent_delta as f64 / time_delta_secs,
            forward_hop_packets_dropped_sec: forward_hop_packets_dropped_delta as f64
                / time_delta_secs,
        }
    }
}

impl From<&EgressMixingStats> for LastMixnetEgress {
    fn from(value: &EgressMixingStats) -> Self {
        LastMixnetEgress {
            forward_hop_packets_sent: value.forward_hop_packets_sent(),
            ack_packets_sent: value.ack_packets_sent(),
            forward_hop_packets_dropped: value.forward_hop_packets_dropped(),
        }
    }
}

#[derive(Debug, Default)]
struct LastWireguard {
    bytes_tx: usize,
    bytes_rx: usize,
}

impl LastWireguard {
    fn rates(&self, previous: &Self, time_delta_secs: f64) -> WireguardRateSinceUpdate {
        let bytes_tx_delta = self.bytes_tx - previous.bytes_tx;
        let bytes_rx_delta = self.bytes_rx - previous.bytes_rx;

        WireguardRateSinceUpdate {
            bytes_tx_sec: bytes_tx_delta as f64 / time_delta_secs,
            bytes_rx_sec: bytes_rx_delta as f64 / time_delta_secs,
        }
    }
}

impl From<&WireguardStats> for LastWireguard {
    fn from(value: &WireguardStats) -> Self {
        LastWireguard {
            bytes_tx: value.bytes_tx(),
            bytes_rx: value.bytes_rx(),
        }
    }
}

pub(crate) struct RateSinceUpdate {
    pub(crate) mixnet: MixnetRateSinceUpdate,
    pub(crate) wireguard: WireguardRateSinceUpdate,
}

pub(crate) struct MixnetRateSinceUpdate {
    pub(crate) ingress: MixnetIngressRateSinceUpdate,
    pub(crate) egress: MixnetEgressRateSinceUpdate,
}

pub(crate) struct MixnetIngressRateSinceUpdate {
    pub(crate) forward_hop_packets_received_sec: f64,
    pub(crate) final_hop_packets_received_sec: f64,
    pub(crate) malformed_packets_received_sec: f64,
    pub(crate) excessive_delay_packets_sec: f64,
    pub(crate) forward_hop_packets_dropped_sec: f64,
    pub(crate) final_hop_packets_dropped_sec: f64,
}

pub(crate) struct MixnetEgressRateSinceUpdate {
    pub(crate) forward_hop_packets_sent_sec: f64,
    pub(crate) ack_packets_sent_sec: f64,
    pub(crate) forward_hop_packets_dropped_sec: f64,
}

pub(crate) struct WireguardRateSinceUpdate {
    pub(crate) bytes_tx_sec: f64,
    pub(crate) bytes_rx_sec: f64,
}
