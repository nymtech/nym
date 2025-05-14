// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::node::key_rotation::manager::SphinxKeyManager;
use crate::node::nym_apis_client::NymApisClient;
use crate::node::replay_protection::manager::ReplayProtectionBloomfiltersManager;
use futures::pin_mut;
use nym_task::ShutdownToken;
use nym_validator_client::models::{KeyRotationInfoResponse, KeyRotationState};
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::{interval, sleep, Instant};
use tracing::{error, info, trace, warn};

pub(crate) struct RotationConfig {
    epoch_duration: Duration,
    rotation_state: KeyRotationState,
}

impl RotationConfig {
    fn rotation_lifetime(&self) -> Duration {
        (self.rotation_state.validity_epochs + 1) * self.epoch_duration
    }
}

impl From<KeyRotationInfoResponse> for RotationConfig {
    fn from(value: KeyRotationInfoResponse) -> Self {
        RotationConfig {
            epoch_duration: value.epoch_duration,
            rotation_state: value.key_rotation_state,
        }
    }
}

pub(crate) struct KeyRotationController {
    // regular polling rate to catch any changes in the system config. they shouldn't happen too often
    // so the requests can be sent quite infrequently
    regular_polling_interval: Duration,

    rotation_config: RotationConfig,
    replay_protection_manager: ReplayProtectionBloomfiltersManager,
    client: NymApisClient,
    managed_keys: SphinxKeyManager,
    shutdown_token: ShutdownToken,
}

struct NextAction {
    typ: KeyRotationActionState,
    deadline: OffsetDateTime,
}

impl NextAction {
    fn until_deadline(&self) -> Duration {
        let now = OffsetDateTime::now_utc();
        Duration::try_from(self.deadline - now).unwrap_or_else(|_| {
            // deadline is already in the past
            Duration::from_nanos(0)
        })
    }
}

#[derive(Clone, Copy)]
enum KeyRotationActionState {
    // generate and pre-announce new key to the nym-api(s)
    PreAnnounce { rotation_id: u32 },

    // perform the following exchange
    // primary -> secondary
    // pre_announced -> primary
    SwapDefault,

    // remove the old overlap key and purge associated data like the replay detection bloomfilter
    PurgeOld,
}

impl KeyRotationController {
    pub(crate) fn new(
        config: &Config,
        rotation_config: RotationConfig,
        client: NymApisClient,
        replay_protection_manager: ReplayProtectionBloomfiltersManager,
        managed_keys: SphinxKeyManager,
        shutdown_token: ShutdownToken,
    ) -> Self {
        KeyRotationController {
            regular_polling_interval: config
                .mixnet
                .key_rotation
                .debug
                .rotation_state_poling_interval,
            rotation_config,
            replay_protection_manager,
            client,
            managed_keys,
            shutdown_token,
        }
    }

    async fn determine_next_action(&self) -> NextAction {
        loop {
            if let Some(next) = self.try_determine_next_action().await {
                return next;
            }

            warn!("failed to determine next key rotation action; will try again in 2min");
            sleep(Duration::from_secs(120)).await;
        }
    }

    async fn try_determine_next_action(&self) -> Option<NextAction> {
        let key_rotation_info = self.try_get_key_rotation_info().await?;
        let current_rotation = key_rotation_info.current_key_rotation_id();

        let current_epoch = key_rotation_info.current_absolute_epoch_id;
        let next_rotation_epoch = key_rotation_info.next_rotation_starting_epoch_id();
        let current_rotation_epoch = key_rotation_info.current_rotation_starting_epoch_id();

        let secondary_rotation_id = self.managed_keys.keys.secondary_key_rotation_id();
        let (action, execution_epoch) = match secondary_rotation_id {
            None => {
                // we don't have any secondary key, meaning the next thing we could possibly do is to pre-announce new key
                // an epoch before next rotation
                let rotation_id = current_rotation + 1;

                (
                    KeyRotationActionState::PreAnnounce { rotation_id },
                    next_rotation_epoch - 1,
                )
            }
            Some(id) if id == current_rotation - 1 => {
                // our secondary key is from the previous rotation, meaning the next thing we have to do
                // is to remove it (we have clearly already rotated)
                (KeyRotationActionState::PurgeOld, current_rotation_epoch + 1)
            }
            Some(id) if id == current_rotation => {
                // our secondary key is from the current epoch, meaning (hopefully) we just have gone into the
                // next rotation, and we have to swap it into the primary
                (KeyRotationActionState::SwapDefault, current_rotation_epoch)
            }
            Some(id) if id == current_rotation + 1 => {
                // our secondary key is from the upcoming rotation, meaning it's the pre-announced key, meaning
                // the next thing we have to do is to swap it into the primary
                (KeyRotationActionState::SwapDefault, next_rotation_epoch)
            }
            Some(other) => {
                // this situation should have never occurred, our secondary key is completely unusable,
                // so we should just remove it immediately and try again
                error!("inconsistent secondary key state. it's marked for rotation {other} while the current value is {current_rotation}");
                (KeyRotationActionState::PurgeOld, current_epoch)
            }
        };

        let now = OffsetDateTime::now_utc();
        let since_epoch_start = now - key_rotation_info.current_epoch_start;
        let until_execution_epoch =
            execution_epoch.saturating_sub(current_epoch) * key_rotation_info.epoch_duration;

        Some(NextAction {
            typ: action,
            deadline: now - since_epoch_start + until_execution_epoch,
        })
    }

    async fn try_get_key_rotation_info(&self) -> Option<KeyRotationInfoResponse> {
        let Ok(rotation_info) = self.client.get_key_rotation_info().await else {
            warn!("failed to retrieve key rotation information from ANY nym-api - we might miss configuration changes");
            return None;
        };

        Some(rotation_info)
    }

    async fn pre_announce_new_key(&self, rotation_id: u32) {
        if let Err(err) = self.managed_keys.generate_key_for_new_rotation(rotation_id) {
            error!("failed to generate and store new sphinx key: {err}");
            return;
        };

        if self
            .replay_protection_manager
            .allocate_pre_announced(rotation_id, self.rotation_config.rotation_lifetime())
            .is_err()
        {
            // mutex poisoning - we have to exit
            self.shutdown_token.cancel();
            return;
        }

        // no need to send the information explicitly to nym-apis, as they're scheduled to refresh
        // self-described endpoints of all nodes before the key rotation epoch rolls over
    }

    fn swap_default_key(&self) {
        if let Err(err) = self.managed_keys.rotate_keys() {
            error!("failed to perform sphinx key swap: {err}")
        };
        if self
            .replay_protection_manager
            .promote_pre_announced()
            .is_err()
        {
            // mutex poisoning - we have to exit
            self.shutdown_token.cancel();
            return;
        }
    }

    fn purge_old_rotation_data(&self) {
        if let Err(err) = self.managed_keys.remove_overlap_key() {
            error!("failed to remove old sphinx key: {err}");
        };
        if self.replay_protection_manager.purge_secondary().is_err() {
            // mutex poisoning - we have to exit
            self.shutdown_token.cancel();
            return;
        }
    }

    async fn execute_next_action(&self, action: KeyRotationActionState) {
        match action {
            KeyRotationActionState::PreAnnounce { rotation_id } => {
                self.pre_announce_new_key(rotation_id).await
            }
            KeyRotationActionState::SwapDefault => self.swap_default_key(),
            KeyRotationActionState::PurgeOld => {
                self.purge_old_rotation_data();
            }
        }
    }

    pub(crate) async fn run(&self) {
        info!("starting sphinx key rotation controller");

        let mut polling_interval = interval(self.regular_polling_interval);
        polling_interval.reset();

        let mut next_action = self.determine_next_action().await;
        let state_update_future = sleep(next_action.until_deadline());
        pin_mut!(state_update_future);

        while !self.shutdown_token.is_cancelled() {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("KeyRotationController: Received shutdown");
                    break;
                }
                _ = polling_interval.tick() => {}
                _ = &mut state_update_future => {
                    self.execute_next_action(next_action.typ).await
                }
            }

            next_action = self.determine_next_action().await;
            state_update_future
                .as_mut()
                .reset(Instant::now() + next_action.until_deadline());
        }

        trace!("KeyRotationController: exiting")
    }

    pub(crate) fn start(self) {
        tokio::spawn(async move { self.run().await });
    }
}
