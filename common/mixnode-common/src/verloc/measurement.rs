// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_node_http_api::state::metrics::{SharedVerlocStats, VerlocNodeResult};
use std::mem;
use time::OffsetDateTime;

pub(crate) trait VerlocStatsUpdateExt {
    async fn start_new_measurements(&self, nodes_to_test: usize);

    async fn append_measurement_results(&self, new_data: Vec<VerlocNodeResult>);

    async fn finish_measurements(&self);
}

impl VerlocStatsUpdateExt for SharedVerlocStats {
    async fn start_new_measurements(&self, nodes_to_test: usize) {
        let mut guard = self.write().await;
        guard.previous_run_data = mem::take(&mut guard.current_run_data);
        guard.current_run_data.nodes_tested = nodes_to_test;
    }

    async fn append_measurement_results(&self, mut new_data: Vec<VerlocNodeResult>) {
        let mut write_permit = self.write().await;
        write_permit.current_run_data.results.append(&mut new_data);
        // make sure the data always stays in order.
        // TODO: considering the front of the results is guaranteed to be sorted, should perhaps
        // a non-default sorting algorithm be used?
        write_permit.current_run_data.results.sort()
    }

    async fn finish_measurements(&self) {
        self.write().await.current_run_data.run_finished = Some(OffsetDateTime::now_utc())
    }
}
