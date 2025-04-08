// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::DashMap;
use std::fmt::Display;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use time::OffsetDateTime;

#[derive(Default)]
pub struct MixingStats {
    // updated on each packet
    pub ingress: IngressMixingStats,

    // updated on each packet
    pub egress: EgressMixingStats,

    // updated on a timer
    pub legacy: LegacyMixingStats,
}

impl MixingStats {
    pub fn update_legacy_stats(
        &self,
        received_since_last_update: usize,
        sent_since_last_update: usize,
        dropped_since_last_update: usize,
        update_timestamp: i64,
    ) {
        self.legacy
            .received_since_last_update
            .store(received_since_last_update, Ordering::Relaxed);
        self.legacy
            .sent_since_last_update
            .store(sent_since_last_update, Ordering::Relaxed);
        self.legacy
            .dropped_since_last_update
            .store(dropped_since_last_update, Ordering::Relaxed);

        let old_last = self.legacy.last_update_ts.load(Ordering::Acquire);
        self.legacy
            .previous_update_ts
            .store(old_last, Ordering::Release);
        self.legacy
            .last_update_ts
            .store(update_timestamp, Ordering::Release);
    }

    pub fn ingress_replayed_packet(&self, source: IpAddr) {
        self.ingress
            .replayed_packets_received
            .fetch_add(1, Ordering::Relaxed);
        self.ingress.senders.entry(source).or_default().replayed += 1;
    }

    pub fn ingress_malformed_packet(&self, source: IpAddr) {
        self.ingress
            .malformed_packets_received
            .fetch_add(1, Ordering::Relaxed);
        self.ingress.senders.entry(source).or_default().malformed += 1;
    }

    pub fn ingress_received_forward_packet(&self, source: IpAddr, version: PacketKind) {
        self.ingress
            .forward_hop_packets_received
            .fetch_add(1, Ordering::Relaxed);
        self.ingress
            .senders
            .entry(source)
            .or_default()
            .forward_packets
            .received += 1;
        *self.ingress.received_versions.entry(version).or_default() += 1;
    }

    pub fn ingress_received_final_hop_packet(&self, source: IpAddr, version: PacketKind) {
        self.ingress
            .final_hop_packets_received
            .fetch_add(1, Ordering::Relaxed);
        self.ingress
            .senders
            .entry(source)
            .or_default()
            .final_hop_packets
            .received += 1;
        *self.ingress.received_versions.entry(version).or_default() += 1;
    }

    pub fn ingress_excessive_delay_packet(&self) {
        self.ingress
            .excessive_delay_packets
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn ingress_dropped_forward_packet(&self, source: IpAddr) {
        self.ingress
            .forward_hop_packets_dropped
            .fetch_add(1, Ordering::Relaxed);
        self.ingress
            .senders
            .entry(source)
            .or_default()
            .forward_packets
            .dropped += 1;
    }

    pub fn ingress_dropped_final_hop_packet(&self, source: IpAddr) {
        self.ingress
            .final_hop_packets_dropped
            .fetch_add(1, Ordering::Relaxed);
        self.ingress
            .senders
            .entry(source)
            .or_default()
            .final_hop_packets
            .dropped += 1;
    }

    pub fn egress_sent_forward_packet(&self, target: SocketAddr) {
        self.egress
            .forward_hop_packets_sent
            .fetch_add(1, Ordering::Relaxed);
        self.egress
            .forward_recipients
            .entry(target)
            .or_default()
            .sent += 1;
    }

    pub fn egress_sent_ack(&self) {
        self.egress.ack_packets_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn egress_dropped_forward_packet(&self, target: SocketAddr) {
        self.egress
            .forward_hop_packets_dropped
            .fetch_add(1, Ordering::Relaxed);
        self.egress
            .forward_recipients
            .entry(target)
            .or_default()
            .dropped += 1;
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct EgressRecipientStats {
    pub dropped: usize,
    pub sent: usize,
}

#[derive(Default)]
pub struct EgressMixingStats {
    disk_persisted_packets: AtomicUsize,

    // this includes ACKS!
    forward_hop_packets_sent: AtomicUsize,

    ack_packets_sent: AtomicUsize,

    forward_hop_packets_dropped: AtomicUsize,

    forward_recipients: DashMap<SocketAddr, EgressRecipientStats>,
}

impl EgressMixingStats {
    pub fn add_disk_persisted_packet(&self) {
        self.disk_persisted_packets.fetch_add(1, Ordering::Relaxed);
    }

    pub fn disk_persisted_packets(&self) -> usize {
        self.disk_persisted_packets.load(Ordering::Relaxed)
    }

    pub fn forward_hop_packets_sent(&self) -> usize {
        self.forward_hop_packets_sent.load(Ordering::Relaxed)
    }

    pub fn ack_packets_sent(&self) -> usize {
        self.ack_packets_sent.load(Ordering::Relaxed)
    }

    pub fn forward_hop_packets_dropped(&self) -> usize {
        self.forward_hop_packets_dropped.load(Ordering::Relaxed)
    }

    pub fn forward_recipients(&self) -> &DashMap<SocketAddr, EgressRecipientStats> {
        &self.forward_recipients
    }

    pub fn remove_stale_forward_recipient(&self, recipient: SocketAddr) {
        self.forward_recipients.remove(&recipient);
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct IngressPacketsStats {
    pub dropped: usize,
    pub received: usize,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct IngressRecipientStats {
    pub forward_packets: IngressPacketsStats,
    pub final_hop_packets: IngressPacketsStats,
    pub malformed: usize,
    pub replayed: usize,
}

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
pub enum PacketKind {
    #[default]
    Unknown,
    Outfox,
    Sphinx(u16),
}

impl Display for PacketKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            PacketKind::Unknown => "unknown".fmt(f),
            PacketKind::Outfox => "outfox".fmt(f),
            PacketKind::Sphinx(sphinx_version) => {
                write!(f, "sphinx_{sphinx_version}")
            }
        }
    }
}

#[derive(Default)]
pub struct IngressMixingStats {
    received_versions: DashMap<PacketKind, i64>,

    // forward hop packets (i.e. to mixnode)
    forward_hop_packets_received: AtomicUsize,

    // final hop packets (i.e. to gateway)
    final_hop_packets_received: AtomicUsize,

    // packets that failed to get unwrapped
    malformed_packets_received: AtomicUsize,

    // packets that were already received and processed before
    replayed_packets_received: AtomicUsize,

    // (forward) packets that had invalid, i.e. too large, delays
    excessive_delay_packets: AtomicUsize,

    // forward hop packets (i.e. to mixnode)
    forward_hop_packets_dropped: AtomicUsize,

    // final hop packets (i.e. to gateway)
    final_hop_packets_dropped: AtomicUsize,

    senders: DashMap<IpAddr, IngressRecipientStats>,
}

impl IngressMixingStats {
    pub fn forward_hop_packets_received(&self) -> usize {
        self.forward_hop_packets_received.load(Ordering::Relaxed)
    }

    pub fn final_hop_packets_received(&self) -> usize {
        self.final_hop_packets_received.load(Ordering::Relaxed)
    }

    pub fn replayed_packets_received(&self) -> usize {
        self.replayed_packets_received.load(Ordering::Relaxed)
    }

    pub fn malformed_packets_received(&self) -> usize {
        self.malformed_packets_received.load(Ordering::Relaxed)
    }

    pub fn excessive_delay_packets(&self) -> usize {
        self.excessive_delay_packets.load(Ordering::Relaxed)
    }

    pub fn forward_hop_packets_dropped(&self) -> usize {
        self.forward_hop_packets_dropped.load(Ordering::Relaxed)
    }

    pub fn final_hop_packets_dropped(&self) -> usize {
        self.final_hop_packets_dropped.load(Ordering::Relaxed)
    }

    pub fn senders(&self) -> &DashMap<IpAddr, IngressRecipientStats> {
        &self.senders
    }

    pub fn packet_versions(&self) -> &DashMap<PacketKind, i64> {
        &self.received_versions
    }

    pub fn remove_stale_sender(&self, sender: IpAddr) {
        self.senders.remove(&sender);
    }
}

#[derive(Debug, Default)]
pub struct LegacyMixingStats {
    last_update_ts: AtomicI64,
    previous_update_ts: AtomicI64,

    received_since_last_update: AtomicUsize,

    // note: sent does not imply forwarded. We don't know if it was delivered successfully
    sent_since_last_update: AtomicUsize,

    // we know for sure we dropped those packets
    dropped_since_last_update: AtomicUsize,
}

impl LegacyMixingStats {
    pub fn last_update(&self) -> OffsetDateTime {
        // SAFETY: all values put here are guaranteed to be valid timestamps
        #[allow(clippy::unwrap_used)]
        OffsetDateTime::from_unix_timestamp(self.last_update_ts.load(Ordering::Relaxed)).unwrap()
    }

    pub fn previous_update(&self) -> OffsetDateTime {
        // SAFETY: all values put here are guaranteed to be valid timestamps
        #[allow(clippy::unwrap_used)]
        OffsetDateTime::from_unix_timestamp(self.previous_update_ts.load(Ordering::Relaxed))
            .unwrap()
    }

    pub fn received_since_last_update(&self) -> usize {
        self.received_since_last_update.load(Ordering::Relaxed)
    }

    pub fn sent_since_last_update(&self) -> usize {
        self.sent_since_last_update.load(Ordering::Relaxed)
    }

    pub fn dropped_since_last_update(&self) -> usize {
        self.dropped_since_last_update.load(Ordering::Relaxed)
    }
}
