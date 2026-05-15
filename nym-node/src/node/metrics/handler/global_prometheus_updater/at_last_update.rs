// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_node_metrics::NymNodeMetrics;
use nym_node_metrics::mixnet::{EgressMixingStats, IngressMixingStats, LpMixingStats, MixingStats};
use nym_node_metrics::wireguard::WireguardStats;
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
    lp: LastLpMixnet,
}

impl LastMixnet {
    fn rates(&self, previous: &Self, time_delta_secs: f64) -> MixnetRateSinceUpdate {
        MixnetRateSinceUpdate {
            ingress: self.ingres.rates(&previous.ingres, time_delta_secs),
            egress: self.egress.rates(&previous.egress, time_delta_secs),
            lp: self.lp.rates(&previous.lp, time_delta_secs),
        }
    }
}

impl From<&MixingStats> for LastMixnet {
    fn from(value: &MixingStats) -> Self {
        LastMixnet {
            ingres: (&value.ingress).into(),
            egress: (&value.egress).into(),
            lp: (&value.lp).into(),
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
struct LastLpMixnet {
    packets_received: usize,
    packets_forwarded: usize,
    routing_filter_dropped: usize,
    messages_received: usize,
    messages_processed: usize,
    malformed_packets: usize,
    excessive_delay_packets: usize,
    replayed_packets: usize,
    final_hop_packets_dropped: usize,
    processing_misc_errors: usize,
    pipeline_overloaded_dropped: usize,
    worker_pool_overloaded_dropped: usize,
    egress_overloaded_dropped: usize,
}

impl LastLpMixnet {
    fn rates(&self, previous: &Self, time_delta_secs: f64) -> LpMixnetRateSinceUpdate {
        let per_sec = |current: usize, previous: usize| -> f64 {
            (current - previous) as f64 / time_delta_secs
        };

        LpMixnetRateSinceUpdate {
            packets_received_sec: per_sec(self.packets_received, previous.packets_received),
            packets_forwarded_sec: per_sec(self.packets_forwarded, previous.packets_forwarded),
            routing_filter_dropped_sec: per_sec(
                self.routing_filter_dropped,
                previous.routing_filter_dropped,
            ),
            messages_received_sec: per_sec(self.messages_received, previous.messages_received),
            messages_processed_sec: per_sec(self.messages_processed, previous.messages_processed),
            malformed_packets_sec: per_sec(self.malformed_packets, previous.malformed_packets),
            excessive_delay_packets_sec: per_sec(
                self.excessive_delay_packets,
                previous.excessive_delay_packets,
            ),
            replayed_packets_sec: per_sec(self.replayed_packets, previous.replayed_packets),
            final_hop_packets_dropped_sec: per_sec(
                self.final_hop_packets_dropped,
                previous.final_hop_packets_dropped,
            ),
            processing_misc_errors_sec: per_sec(
                self.processing_misc_errors,
                previous.processing_misc_errors,
            ),
            pipeline_overloaded_dropped_sec: per_sec(
                self.pipeline_overloaded_dropped,
                previous.pipeline_overloaded_dropped,
            ),
            worker_pool_overloaded_dropped_sec: per_sec(
                self.worker_pool_overloaded_dropped,
                previous.worker_pool_overloaded_dropped,
            ),
            egress_overloaded_dropped_sec: per_sec(
                self.egress_overloaded_dropped,
                previous.egress_overloaded_dropped,
            ),
        }
    }
}

impl From<&LpMixingStats> for LastLpMixnet {
    fn from(value: &LpMixingStats) -> Self {
        LastLpMixnet {
            packets_received: value.packets_received(),
            packets_forwarded: value.packets_forwarded(),
            routing_filter_dropped: value.routing_filter_dropped(),
            messages_received: value.messages_received(),
            messages_processed: value.messages_processed(),
            malformed_packets: value.malformed_packets(),
            excessive_delay_packets: value.excessive_delay_packets(),
            replayed_packets: value.replayed_packets(),
            final_hop_packets_dropped: value.final_hop_packets_dropped(),
            processing_misc_errors: value.processing_misc_errors(),
            pipeline_overloaded_dropped: value.pipeline_overloaded_dropped_packets(),
            worker_pool_overloaded_dropped: value.worker_pool_overloaded_dropped_packets(),
            egress_overloaded_dropped: value.egress_overloaded_dropped_packets(),
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
    pub(crate) lp: LpMixnetRateSinceUpdate,
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

pub(crate) struct LpMixnetRateSinceUpdate {
    pub(crate) packets_received_sec: f64,
    pub(crate) packets_forwarded_sec: f64,
    pub(crate) routing_filter_dropped_sec: f64,
    pub(crate) messages_received_sec: f64,
    pub(crate) messages_processed_sec: f64,
    pub(crate) malformed_packets_sec: f64,
    pub(crate) excessive_delay_packets_sec: f64,
    pub(crate) replayed_packets_sec: f64,
    pub(crate) final_hop_packets_dropped_sec: f64,
    pub(crate) processing_misc_errors_sec: f64,
    pub(crate) pipeline_overloaded_dropped_sec: f64,
    pub(crate) worker_pool_overloaded_dropped_sec: f64,
    pub(crate) egress_overloaded_dropped_sec: f64,
}

pub(crate) struct WireguardRateSinceUpdate {
    pub(crate) bytes_tx_sec: f64,
    pub(crate) bytes_rx_sec: f64,
}
