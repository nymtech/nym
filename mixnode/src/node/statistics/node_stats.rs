// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, RwLockReadGuard};

use super::packet_event_reporter::{PacketsMap, SharedCurrentPacketEvents};

// The main type exposing stats to the rest of the crate. It is periodically updated to accumulate
// total stats.
#[derive(Clone)]
pub struct SharedNodeStats {
    inner: Arc<RwLock<NodeStats>>,
}

impl SharedNodeStats {
    pub fn new() -> Self {
        let now = SystemTime::now();
        SharedNodeStats {
            inner: Arc::new(RwLock::new(NodeStats {
                update_time: now,
                previous_update_time: now,
                packets_received_since_startup: 0,
                packets_sent_since_startup: HashMap::new(),
                packets_explicitly_dropped_since_startup: HashMap::new(),
                packets_received_since_last_update: 0,
                packets_sent_since_last_update: HashMap::new(),
                packets_explicitly_dropped_since_last_update: HashMap::new(),
            })),
        }
    }

    async fn update(&self, new_received: u64, new_sent: PacketsMap, new_dropped: PacketsMap) {
        let mut guard = self.inner.write().await;
        let snapshot_time = SystemTime::now();

        guard.previous_update_time = guard.update_time;
        guard.update_time = snapshot_time;

        guard.packets_received_since_startup += new_received;
        for (mix, count) in new_sent.iter() {
            *guard
                .packets_sent_since_startup
                .entry(mix.clone())
                .or_insert(0) += *count;
        }

        for (mix, count) in new_dropped.iter() {
            *guard
                .packets_explicitly_dropped_since_last_update
                .entry(mix.clone())
                .or_insert(0) += *count;
        }

        guard.packets_received_since_last_update = new_received;
        guard.packets_sent_since_last_update = new_sent;
        guard.packets_explicitly_dropped_since_last_update = new_dropped;
    }

    pub(crate) async fn clone_data(&self) -> NodeStats {
        self.inner.read().await.clone()
    }

    pub(super) async fn read(&self) -> RwLockReadGuard<'_, NodeStats> {
        self.inner.read().await
    }
}

#[derive(Serialize, Clone)]
pub struct NodeStats {
    #[serde(serialize_with = "humantime_serde::serialize")]
    pub update_time: SystemTime,

    #[serde(serialize_with = "humantime_serde::serialize")]
    pub previous_update_time: SystemTime,

    pub packets_received_since_startup: u64,

    // note: sent does not imply forwarded. We don't know if it was delivered successfully
    pub packets_sent_since_startup: PacketsMap,

    // we know for sure we dropped packets to those destinations
    pub packets_explicitly_dropped_since_startup: PacketsMap,

    pub packets_received_since_last_update: u64,

    // note: sent does not imply forwarded. We don't know if it was delivered successfully
    pub packets_sent_since_last_update: PacketsMap,

    // we know for sure we dropped packets to those destinations
    pub packets_explicitly_dropped_since_last_update: PacketsMap,
}

impl NodeStats {
    pub fn simplify(&self) -> NodeStatsSimple {
        NodeStatsSimple {
            update_time: self.update_time,
            previous_update_time: self.previous_update_time,
            packets_received_since_startup: self.packets_received_since_startup,
            packets_sent_since_startup: self.packets_sent_since_startup.values().sum(),
            packets_explicitly_dropped_since_startup: self
                .packets_explicitly_dropped_since_startup
                .values()
                .sum(),
            packets_received_since_last_update: self.packets_received_since_last_update,
            packets_sent_since_last_update: self.packets_sent_since_last_update.values().sum(),
            packets_explicitly_dropped_since_last_update: self
                .packets_explicitly_dropped_since_last_update
                .values()
                .sum(),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct NodeStatsSimple {
    #[serde(serialize_with = "humantime_serde::serialize")]
    update_time: SystemTime,

    #[serde(serialize_with = "humantime_serde::serialize")]
    previous_update_time: SystemTime,

    packets_received_since_startup: u64,

    // note: sent does not imply forwarded. We don't know if it was delivered successfully
    packets_sent_since_startup: u64,

    // we know for sure we dropped those packets
    packets_explicitly_dropped_since_startup: u64,

    packets_received_since_last_update: u64,

    // note: sent does not imply forwarded. We don't know if it was delivered successfully
    packets_sent_since_last_update: u64,

    // we know for sure we dropped those packets
    packets_explicitly_dropped_since_last_update: u64,
}

pub struct SharedStatsUpdater {
    updating_delay: Duration,
    current_packet_data: SharedCurrentPacketEvents,
    shared_stats: SharedNodeStats,
}

impl SharedStatsUpdater {
    pub fn new(
        updating_delay: Duration,
        current_packet_data: SharedCurrentPacketEvents,
        current_stats: SharedNodeStats,
    ) -> Self {
        SharedStatsUpdater {
            updating_delay,
            current_packet_data,
            shared_stats: current_stats,
        }
    }

    async fn update_stats(&self) {
        // grab new data since last update
        let (received, sent, dropped) = self.current_packet_data.acquire_and_reset().await;
        self.shared_stats.update(received, sent, dropped).await;
    }

    pub async fn run(&self) {
        loop {
            tokio::time::sleep(self.updating_delay).await;
            self.update_stats().await
        }
    }
}
