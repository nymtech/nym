// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::state::AppState;
use axum::extract::FromRef;
use nym_node_requests::api::v1::metrics::models::{
    MixingStats, SessionStats, VerlocResult, VerlocResultData, VerlocStats,
};
use std::collections::HashMap;
use std::sync::Arc;
use time::macros::time;
use time::{Date, OffsetDateTime};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub use nym_node_requests::api::v1::metrics::models::{VerlocMeasurement, VerlocNodeResult};

type PacketsMap = HashMap<String, u64>;

#[derive(Clone, Debug, Default)]
pub struct SharedMixingStats {
    inner: Arc<RwLock<MixingStatsState>>,
}

impl SharedMixingStats {
    pub fn new() -> SharedMixingStats {
        let now = OffsetDateTime::now_utc();

        SharedMixingStats {
            inner: Arc::new(RwLock::new(MixingStatsState {
                update_time: now,
                previous_update_time: now,
                ..Default::default()
            })),
        }
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, MixingStatsState> {
        self.inner.read().await
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, MixingStatsState> {
        self.inner.write().await
    }
}

#[derive(Debug)]
pub struct MixingStatsState {
    pub update_time: OffsetDateTime,

    pub previous_update_time: OffsetDateTime,

    pub packets_received_since_startup: u64,
    pub packets_sent_since_startup_all: u64,
    pub packets_dropped_since_startup_all: u64,
    pub packets_received_since_last_update: u64,

    // note: sent does not imply forwarded. We don't know if it was delivered successfully
    pub packets_sent_since_last_update: PacketsMap,

    // we know for sure we dropped packets to those destinations
    pub packets_explicitly_dropped_since_last_update: PacketsMap,
}

impl MixingStatsState {
    pub fn as_response(&self) -> MixingStats {
        MixingStats {
            update_time: self.update_time,
            previous_update_time: self.previous_update_time,
            received_since_startup: self.packets_received_since_startup,
            sent_since_startup: self.packets_sent_since_startup_all,
            dropped_since_startup: self.packets_dropped_since_startup_all,
            received_since_last_update: self.packets_received_since_last_update,
            sent_since_last_update: self.packets_sent_since_last_update.values().sum(),
            dropped_since_last_update: self
                .packets_explicitly_dropped_since_last_update
                .values()
                .sum(),
        }
    }
}

impl Default for MixingStatsState {
    fn default() -> Self {
        MixingStatsState {
            update_time: OffsetDateTime::UNIX_EPOCH,
            previous_update_time: OffsetDateTime::UNIX_EPOCH,
            packets_received_since_startup: 0,
            packets_sent_since_startup_all: 0,
            packets_dropped_since_startup_all: 0,
            packets_received_since_last_update: 0,
            packets_sent_since_last_update: Default::default(),
            packets_explicitly_dropped_since_last_update: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SharedVerlocStats {
    inner: Arc<RwLock<VerlocStatsState>>,
}

#[derive(Clone, Debug, Default)]
pub struct VerlocStatsState {
    pub current_run_data: VerlocResultData,
    pub previous_run_data: VerlocResultData,
}

impl SharedVerlocStats {
    pub async fn read(&self) -> RwLockReadGuard<'_, VerlocStatsState> {
        self.inner.read().await
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, VerlocStatsState> {
        self.inner.write().await
    }
}

impl VerlocStatsState {
    pub fn as_response(&self) -> VerlocStats {
        let previous = if !self.previous_run_data.run_finished() {
            VerlocResult::Unavailable
        } else {
            VerlocResult::Data(self.previous_run_data.clone())
        };

        let current = if !self.current_run_data.run_finished() {
            VerlocResult::MeasurementInProgress
        } else {
            VerlocResult::Data(self.current_run_data.clone())
        };

        VerlocStats { previous, current }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SharedSessionStats {
    inner: Arc<RwLock<SessionStatsState>>,
}

impl SharedSessionStats {
    pub fn new() -> SharedSessionStats {
        SharedSessionStats {
            inner: Arc::new(RwLock::new(Default::default())),
        }
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, SessionStatsState> {
        self.inner.read().await
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, SessionStatsState> {
        self.inner.write().await
    }
}

#[derive(Debug, Clone)]
pub struct SessionStatsState {
    pub update_time: Date,
    pub unique_active_users: u32,
    pub session_started: u32,
    pub session_durations: Vec<u64>,
}

impl SessionStatsState {
    pub fn as_response(&self) -> SessionStats {
        SessionStats {
            update_time: self.update_time.with_time(time!(0:00)).assume_utc(),
            unique_active_users: self.unique_active_users,
            session_durations: self.session_durations.clone(),
            sessions_started: self.session_started,
            sessions_finished: self.session_durations.len() as u32,
        }
    }
}

impl Default for SessionStatsState {
    fn default() -> Self {
        SessionStatsState {
            update_time: OffsetDateTime::UNIX_EPOCH.date(),
            unique_active_users: 0,
            session_started: 0,
            session_durations: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MetricsAppState {
    pub(crate) prometheus_access_token: Option<String>,

    pub(crate) mixing_stats: SharedMixingStats,

    pub(crate) session_stats: SharedSessionStats,

    pub(crate) verloc: SharedVerlocStats,
}

impl FromRef<AppState> for MetricsAppState {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.metrics.clone()
    }
}
