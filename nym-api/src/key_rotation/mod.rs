// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::caching::refresher::{CacheUpdateWatcher, RefreshRequester};
use nym_mixnet_contract_common::{Interval, KeyRotationState};
use nym_task::TaskClient;
use time::OffsetDateTime;
use tracing::{debug, error, info, trace};

#[derive(Debug)]
struct ContractData {
    interval: Interval,
    key_rotation_state: KeyRotationState,
}

impl ContractData {
    fn rotation_id(&self) -> u32 {
        self.key_rotation_state
            .key_rotation_id(self.interval.current_epoch_absolute_id())
    }

    fn upcoming_rotation_id(&self) -> u32 {
        self.rotation_id() + 1
    }

    fn current_epoch_progress(&self, now: OffsetDateTime) -> f32 {
        let elapsed = (now - self.interval.current_epoch_start()).as_seconds_f32();
        elapsed / self.interval.epoch_length().as_secs_f32()
    }

    fn epochs_until_next_rotation(&self) -> Option<f32> {
        let current_epoch_progress = self.current_epoch_progress(OffsetDateTime::now_utc());

        if !(0. ..=1.).contains(&current_epoch_progress) {
            error!("epoch seems to be stuck (current progress is at {:.1}%) - can't progress key rotation!", current_epoch_progress * 100.);
            return None;
        }

        let next_rotation_epoch = self
            .key_rotation_state
            .next_rotation_starting_epoch_id(self.interval.current_epoch_absolute_id());

        let Some(full_epochs) =
            (next_rotation_epoch - self.interval.current_epoch_absolute_id()).checked_sub(1)
        else {
            error!("CRITICAL FAILURE: invalid epoch calculation");
            return None;
        };

        Some((1. - current_epoch_progress) + full_epochs as f32)
    }
}

// 'simple' task responsible for making sure nym-api refreshes its self-described cache
// just before the next key rotation so it would have all the keys available
pub(crate) struct KeyRotationController {
    pub(crate) last_described_refreshed_for: Option<u32>,

    pub(crate) describe_cache_refresher: RefreshRequester,
    pub(crate) contract_cache_watcher: CacheUpdateWatcher,
    pub(crate) contract_cache: NymContractCache,
}

impl KeyRotationController {
    pub(crate) fn new(
        describe_cache_refresher: RefreshRequester,
        contract_cache_watcher: CacheUpdateWatcher,
        contract_cache: NymContractCache,
    ) -> KeyRotationController {
        KeyRotationController {
            last_described_refreshed_for: None,
            describe_cache_refresher,
            contract_cache_watcher,
            contract_cache,
        }
    }

    // SAFETY: this function is only called after cache has already been initialised
    #[allow(clippy::unwrap_used)]
    async fn get_contract_data(&self) -> ContractData {
        let key_rotation_state = self.contract_cache.get_key_rotation_state().await.unwrap();
        let interval = self.contract_cache.current_interval().await.unwrap();
        ContractData {
            interval,
            key_rotation_state,
        }
    }

    async fn handle_contract_cache_update(&mut self) {
        let updated = self.get_contract_data().await;

        debug!(
            "current key rotation: {}",
            updated
                .key_rotation_state
                .key_rotation_id(updated.interval.current_epoch_absolute_id())
        );

        // if we're only 1/4 epoch away from the next rotation, and we haven't yet performed the refresh,
        // update the self-described cache, as all nodes should have already pre-announced their new sphinx keys
        if let Some(remaining) = updated.epochs_until_next_rotation() {
            debug!("{remaining} epoch(s) remaining until next key rotation");
            let expected = Some(updated.upcoming_rotation_id());
            if remaining < 0.25 && self.last_described_refreshed_for != expected {
                info!("{remaining} epoch(s) remaining until next key rotation - requesting full refresh of self-described cache");
                self.describe_cache_refresher.request_cache_refresh();
                self.last_described_refreshed_for = expected;
            }
        }
    }

    async fn run(&mut self, mut task_client: TaskClient) {
        self.contract_cache.naive_wait_for_initial_values().await;
        self.handle_contract_cache_update().await;

        while !task_client.is_shutdown() {
            tokio::select! {
                biased;
                _ = task_client.recv() => {
                    trace!("KeyRotationController: Received shutdown");
                }
                _ = self.contract_cache_watcher.changed() => {
                    self.handle_contract_cache_update().await
                }
            }
        }

        trace!("KeyRotationController: exiting")
    }

    pub(crate) fn start(mut self, task_client: TaskClient) {
        tokio::spawn(async move { self.run(task_client).await });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::Timestamp;
    use std::time::Duration;

    // Sun Jun 15 2025 15:06:40 GMT+0000
    const DUMMY_TIMESTAMP: i64 = 1750000000;

    fn dummy_contract_data() -> ContractData {
        let mut env = mock_env();

        env.block.time = Timestamp::from_seconds(DUMMY_TIMESTAMP as u64);
        ContractData {
            interval: Interval::init_interval(24, Duration::from_secs(60 * 60), &env),
            key_rotation_state: KeyRotationState {
                validity_epochs: 0,
                initial_epoch_id: 0,
            },
        }
    }

    #[test]
    fn current_epoch_progress() {
        let dummy_data = dummy_contract_data();

        let epoch_start = OffsetDateTime::from_unix_timestamp(DUMMY_TIMESTAMP).unwrap();
        let quarter_in = OffsetDateTime::from_unix_timestamp(DUMMY_TIMESTAMP + 15 * 60).unwrap();
        let half_in = OffsetDateTime::from_unix_timestamp(DUMMY_TIMESTAMP + 30 * 60).unwrap();
        let next = OffsetDateTime::from_unix_timestamp(DUMMY_TIMESTAMP + 60 * 60).unwrap();
        let one_and_half = OffsetDateTime::from_unix_timestamp(DUMMY_TIMESTAMP + 90 * 60).unwrap();
        let past_value = OffsetDateTime::from_unix_timestamp(DUMMY_TIMESTAMP - 30 * 60).unwrap();

        assert_eq!(dummy_data.current_epoch_progress(epoch_start), 0.);
        assert_eq!(dummy_data.current_epoch_progress(quarter_in), 0.25);
        assert_eq!(dummy_data.current_epoch_progress(half_in), 0.5);
        assert_eq!(dummy_data.current_epoch_progress(next), 1.);
        assert_eq!(dummy_data.current_epoch_progress(one_and_half), 1.5);
        assert_eq!(dummy_data.current_epoch_progress(past_value), -0.5);
    }
}
