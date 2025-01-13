// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use arc_swap::ArcSwap;
use nym_node_requests::api::v1::node_load::models::{Load, NodeLoad};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, System};
use time::OffsetDateTime;
use tracing::debug;

#[derive(Clone)]
pub struct CachedNodeLoad {
    ttl: Duration,
    being_updated: Arc<AtomicBool>,
    inner: Arc<ArcSwap<CachedNodeLoadInner>>,
}

impl CachedNodeLoad {
    pub(crate) fn new(ttl: Duration) -> Self {
        CachedNodeLoad {
            ttl,
            being_updated: Arc::new(Default::default()),
            inner: Arc::new(ArcSwap::new(Arc::new(CachedNodeLoadInner::initial()))),
        }
    }

    pub(crate) fn current_load(&self) -> NodeLoad {
        let now = OffsetDateTime::now_utc();

        let inner = self.inner.load();
        if inner.timestamp + self.ttl < now {
            // new
            let already_being_updated = self.being_updated.swap(true, Ordering::SeqCst);
            if already_being_updated {
                // use the 'stale' entry because it is already being updated by another thread
                inner.load
            } else {
                self.update_cache()
            }
        } else {
            inner.load
        }
    }

    fn update_cache(&self) -> NodeLoad {
        let old = self.inner.load();
        let current_load = CachedNodeLoadInner::update(&old);
        let load = current_load.load;
        self.inner.store(Arc::new(current_load));
        self.being_updated.store(false, Ordering::SeqCst);
        load
    }
}

pub struct CachedNodeLoadInner {
    timestamp: OffsetDateTime,
    last_transmitted: u64,
    last_received: u64,
    load: NodeLoad,
}

struct RawUsage {
    load_per_cpu: f64,
    memory_usage: f64,
    swap_usage: f64,
    eth_transmitted: u64,
    eth_received: u64,
    total_swap: u64,
}

impl RawUsage {
    fn current() -> RawUsage {
        let mut system = sysinfo::System::new();
        let networks = Networks::new_with_refreshed_list();

        system.refresh_memory_specifics(MemoryRefreshKind::everything());
        system.refresh_cpu_specifics(CpuRefreshKind::nothing());

        let average_load = System::load_average();
        let cpu_count = system.cpus().len();

        let load_per_cpu = average_load.five / cpu_count as f64;

        let total_memory = system.total_memory();
        let used_memory = system.used_memory();

        let memory_usage = used_memory as f64 / total_memory as f64;

        let total_swap = system.total_swap();
        let used_swap = system.free_swap();

        let swap_usage = used_swap as f64 / total_swap as f64;

        let mut eth_transmitted = 0;
        let mut eth_received = 0;

        // we're only interested in interfaces with 'eth' or 'en' prefix
        // (that's a very weak assumption, but that's just first iteration of this endpoint)
        for (interface, data) in networks.list() {
            if interface.starts_with("eth") || interface.starts_with("en") {
                eth_transmitted += data.total_transmitted();
                eth_received += data.total_received();
            }
        }

        debug!(average_load = ?average_load, memory_usage=memory_usage, swap_usage = swap_usage, "current load");

        RawUsage {
            load_per_cpu,
            memory_usage,
            swap_usage,
            eth_transmitted,
            eth_received,
            total_swap,
        }
    }
}

impl CachedNodeLoadInner {
    pub fn initial() -> CachedNodeLoadInner {
        let timestamp = OffsetDateTime::now_utc();
        let raw_usage = RawUsage::current();

        let mut base_load = Load::from(raw_usage.load_per_cpu);
        let memory_load = Load::from(raw_usage.memory_usage);

        // if memory load is of higher tier, increment the base load by one level
        // (i.e. for example from 'Low' to 'Medium')
        if memory_load > base_load {
            base_load = base_load.increment();
        }

        if raw_usage.total_swap > 1024 * 1024 * 1024 {
            // same with swap
            let swap_load = Load::from(raw_usage.swap_usage);
            if swap_load > base_load {
                base_load = base_load.increment();
            }
        }

        CachedNodeLoadInner {
            timestamp,
            last_transmitted: raw_usage.eth_transmitted,
            last_received: raw_usage.eth_received,
            load: NodeLoad {
                total: base_load,
                machine: base_load,
                network: Load::Unknown,
            },
        }
    }

    pub fn update(previous: &Self) -> CachedNodeLoadInner {
        let timestamp = OffsetDateTime::now_utc();
        let raw_usage = RawUsage::current();

        let time_delta = (timestamp - previous.timestamp).as_seconds_f64();
        let tx_delta = raw_usage.eth_transmitted - previous.last_transmitted;
        let rx_delta = raw_usage.eth_received - previous.last_received;

        let tx_bs = tx_delta as f64 / time_delta;
        let rx_bs = rx_delta as f64 / time_delta;

        // currently we consider value of 1Gbps to be maximum load
        // in the future we should allow specifying custom sizes of network cards
        // (Gbps = Bytes/s * 0.000000008)
        let tx_gbps = tx_bs * 0.000000008;
        let rx_gbps = rx_bs * 0.000000008;
        let network_load = Load::from(tx_gbps.max(rx_gbps));

        debug!(tx_gbps = tx_gbps, rx_gbps = rx_gbps, "network load");

        let mut base_load = Load::from(raw_usage.load_per_cpu);
        let memory_load = Load::from(raw_usage.memory_usage);

        // if memory load is of higher tier, increment the base load by one level
        // (i.e. for example from 'Low' to 'Medium')
        if memory_load > base_load {
            base_load = base_load.increment();
        }

        if raw_usage.total_swap > 1024 * 1024 * 1024 {
            // same with swap
            let swap_load = Load::from(raw_usage.swap_usage);
            if swap_load > base_load {
                base_load = base_load.increment();
            }
        }

        let total_load = if base_load > network_load {
            base_load
        } else {
            base_load.increment()
        };

        CachedNodeLoadInner {
            timestamp,
            last_transmitted: raw_usage.eth_transmitted,
            last_received: raw_usage.eth_received,
            load: NodeLoad {
                total: total_load,
                machine: base_load,
                network: network_load,
            },
        }
    }
}
