// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{VerlocNodeResult, VerlocResultData};
use std::mem;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Clone, Debug, Default)]
pub struct SharedVerlocStats {
    inner: Arc<RwLock<VerlocStatsState>>,
}

impl SharedVerlocStats {
    pub(crate) async fn start_new_measurements(&self, nodes_to_test: usize) {
        let mut guard = self.write().await;
        guard.previous_run_data = mem::take(&mut guard.current_run_data);
        guard.current_run_data.nodes_tested = nodes_to_test;
    }

    pub(crate) async fn append_measurement_results(&self, mut new_data: Vec<VerlocNodeResult>) {
        let mut write_permit = self.write().await;
        write_permit.current_run_data.results.append(&mut new_data);
        // make sure the data always stays in order.
        // TODO: considering the front of the results is guaranteed to be sorted, should perhaps
        // a non-default sorting algorithm be used?
        write_permit.current_run_data.results.sort()
    }

    pub(crate) async fn finish_measurements(&self) {
        self.write().await.current_run_data.run_finished = Some(OffsetDateTime::now_utc())
    }
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
