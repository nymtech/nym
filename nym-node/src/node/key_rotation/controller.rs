// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::node::key_rotation::manager::SphinxKeyManager;
use crate::node::nym_apis_client::NymApisClient;
use crate::node::replay_protection::manager::ReplayProtectionBloomfiltersManager;
use futures::pin_mut;
use nym_task::ShutdownToken;
use nym_validator_client::models::{KeyRotationDetails, KeyRotationInfoResponse, KeyRotationState};
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::{interval, sleep, Instant};
use tracing::{debug, error, info, trace, warn};

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
            epoch_duration: value.details.epoch_duration,
            rotation_state: value.details.key_rotation_state,
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
    fn new(typ: KeyRotationActionState, deadline: OffsetDateTime) -> Self {
        NextAction { typ, deadline }
    }

    fn until_deadline(&self) -> Duration {
        let now = OffsetDateTime::now_utc();
        Duration::try_from(self.deadline - now).unwrap_or_else(|_| {
            // deadline is already in the past
            Duration::from_nanos(0)
        })
    }

    fn wait(duration: Duration) -> NextAction {
        NextAction::new(
            KeyRotationActionState::Wait,
            OffsetDateTime::now_utc() + duration,
        )
    }

    fn pre_announce(rotation_id: u32, deadline: OffsetDateTime) -> Self {
        NextAction::new(
            KeyRotationActionState::PreAnnounce { rotation_id },
            deadline,
        )
    }

    fn swap_default(expected_new_rotation: u32, deadline: OffsetDateTime) -> Self {
        NextAction::new(
            KeyRotationActionState::SwapDefault {
                expected_new_rotation,
            },
            deadline,
        )
    }

    fn purge_secondary(deadline: OffsetDateTime) -> Self {
        NextAction::new(KeyRotationActionState::PurgeOld, deadline)
    }
}

#[derive(Debug, Clone, Copy)]
enum KeyRotationActionState {
    // generate and pre-announce new key to the nym-api(s)
    PreAnnounce { rotation_id: u32 },

    // perform the following exchange
    // primary -> secondary
    // pre_announced -> primary
    SwapDefault { expected_new_rotation: u32 },

    // remove the old overlap key and purge associated data like the replay detection bloomfilter
    PurgeOld,

    // a no-op action that has only a single purpose - wait (used to handle slight desyncs)
    Wait,
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

    async fn try_determine_next_action(&self) -> NextAction {
        let now = OffsetDateTime::now_utc();
        let Some(key_rotation_info) = self.try_get_key_rotation_info().await else {
            warn!("failed to retrieve key rotation information");
            return NextAction::wait(Duration::from_secs(240));
        };

        // check if we think the epoch is stuck (we're already 20% or more into following epoch with no advancement)
        if key_rotation_info.is_epoch_stuck() {
            warn!("the epoch is stuck - can't progress with key rotation");
            return NextAction::wait(Duration::from_secs(240));
        }

        // >>>>> START: determine if we called this method pre-maturely due to clock skew

        // current rotation id as determined by the current epoch id
        let current_rotation_id = key_rotation_info.current_key_rotation_id();

        // expected rotation id as determined by the current TIME
        // used to determined epoch stalling or clocks being slightly out of sync
        let expected_current_rotation_id = key_rotation_info.expected_current_rotation_id();

        if current_rotation_id != expected_current_rotation_id {
            warn!("the current rotation is {current_rotation_id} whilst we expected {expected_current_rotation_id}");
            // if we got here, it means epoch is most likely NOT stuck (we're within the threshold)
            // so probably we prematurely called this method before nym-api(s) got to advancing
            // the epoch and thus the rotation, so wait a bit instead.
            return NextAction::wait(Duration::from_secs(30));
        }
        // >>>>> END: determine if we called this method pre-maturely due to clock skew

        // if we're less than 30s until next rotation, we probably started our binary in a rather
        // unfortunate time, just wait until the next rotation rather than do all the work only to throw it
        // away immediately
        let Some(until_next_rotation) = key_rotation_info.until_next_rotation() else {
            debug!("key rotation is overdue - waiting...");
            return NextAction::wait(Duration::from_secs(30));
        };
        if until_next_rotation < Duration::from_secs(30) {
            debug!("less than 30s until next rotation - waiting until then");
            return NextAction::wait(Duration::from_secs(30));
        }

        let current_epoch = key_rotation_info.current_absolute_epoch_id;

        // epoch id of when the current rotation has started
        let current_rotation_start_epoch = key_rotation_info.current_rotation_starting_epoch_id();

        // epoch id of when the new rotation id is meant to start
        let next_rotation_start_epoch = key_rotation_info.next_rotation_starting_epoch_id();

        let secondary_key_rotation_id = self.managed_keys.keys.secondary_key_rotation_id();
        let primary_key_rotation_id = self.managed_keys.keys.primary_key_rotation_id();

        debug!(
            "current rotation: {current_rotation_id}, primary: {}, secondary: {secondary_key_rotation_id:?}",
            self.managed_keys.keys.primary_key_rotation_id()
        );

        let rotates_next_epoch = next_rotation_start_epoch == current_epoch + 1;
        let next_rotation_id = current_rotation_id + 1;

        let Some(secondary_key_rotation_id) = secondary_key_rotation_id else {
            debug!("we don't have a secondary key");
            // figure out if we already have appropriate key (like we crashed or this is the first time node is running)
            // or whether we have to regenerate anything or, which is the most likely case, we're waiting to
            // pre-announce new key for the following rotation

            if primary_key_rotation_id != current_rotation_id {
                warn!("current primary key does not correspond to the current rotation - immediately pre-announcing new key (rotates next epoch: {rotates_next_epoch}");
                // we don't have a secondary key and our current key is already outdated -
                // preannounce a key for either this or the next rotation
                // (and next time this method is called, it will be promoted to primary)
                return if rotates_next_epoch {
                    NextAction::pre_announce(next_rotation_id, now)
                } else {
                    NextAction::pre_announce(current_rotation_id, now)
                };
            }

            // we have a primary key corresponding to the current rotation, so we just have to pre-announce
            // a key for the next rotation an epoch before the rotation
            let deadline = key_rotation_info.epoch_start_time(next_rotation_start_epoch - 1);
            debug!(
                "going to pre-announce secondary key for rotation {next_rotation_id} on {deadline}"
            );
            return NextAction::pre_announce(next_rotation_id, deadline);
        };

        // the current secondary key corresponds to the next rotation, i.e. this is the pre-announced key
        if secondary_key_rotation_id == next_rotation_id {
            debug!("secondary key is for the NEXT rotation - we need to swap into it");

            let deadline = key_rotation_info.epoch_start_time(next_rotation_start_epoch);
            return NextAction::swap_default(next_rotation_id, deadline);
        }

        if secondary_key_rotation_id == current_rotation_id {
            debug!("secondary key is for the CURRENT rotation - we need to swap into it");

            return NextAction::swap_default(current_rotation_id, now);
        }

        if secondary_key_rotation_id < current_rotation_id {
            let deadline = if secondary_key_rotation_id == current_rotation_id - 1 {
                debug!("secondary key is from the PREVIOUS rotations - we need to purge it");
                // we purge the key after the end of overlap period, i.e. during the 2nd epoch of a rotation
                key_rotation_info.epoch_start_time(current_rotation_start_epoch + 1)
            } else {
                debug!("secondary key is from AN OLD rotation - we need to purge it");
                // the key is from some old rotation, we were probably offline for some time - we need to pre-announce new key
                // for the upcoming rotation, so start off by purging this key immediately
                now
            };

            return NextAction::purge_secondary(deadline);
        }

        // at this point all branches should have been covered, i.e. missing secondary key,
        // secondary key == next rotation
        // secondary key == current rotation
        // secondary key < current rotation
        // the only, theoretical, branch is if secondary key was from few rotations in the future,
        // but this would require some weird chain shenanigans
        error!("this code branch should have been unreachable - please report if you see this error with the following information:\
            primary_key_rotation = {primary_key_rotation_id},
            secondary_key_rotation = {secondary_key_rotation_id},
            current_rotation = {current_rotation_id},
            next_rotation = {next_rotation_id},
            raw_response = {key_rotation_info:?}");

        NextAction::wait(Duration::from_secs(240))
    }

    async fn try_get_key_rotation_info(&self) -> Option<KeyRotationDetails> {
        let Ok(rotation_info) = self.client.get_key_rotation_info().await else {
            warn!("failed to retrieve key rotation information from ANY nym-api - we might miss configuration changes");
            return None;
        };

        Some(rotation_info.details)
    }

    async fn pre_announce_new_key(&self, rotation_id: u32) {
        info!("pre-announcing new key for rotation {rotation_id}");
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
        }

        // no need to send the information explicitly to nym-apis, as they're scheduled to refresh
        // self-described endpoints of all nodes before the key rotation epoch rolls over
    }

    fn swap_default_key(&self, expected_new_rotation: u32) {
        info!("attempting to swap the primary key to the previously generated one");
        if let Err(err) = self.managed_keys.rotate_keys(expected_new_rotation) {
            error!("failed to perform sphinx key swap: {err}")
        };
        if self
            .replay_protection_manager
            .promote_pre_announced()
            .is_err()
        {
            // mutex poisoning - we have to exit
            self.shutdown_token.cancel();
        }
    }

    fn purge_old_rotation_data(&self) {
        info!("purging data associated with the old sphinx key");
        if let Err(err) = self.managed_keys.remove_overlap_key() {
            error!("failed to remove old sphinx key: {err}");
        };
        if self.replay_protection_manager.purge_secondary().is_err() {
            // mutex poisoning - we have to exit
            self.shutdown_token.cancel();
        }
    }

    async fn execute_next_action(&self, action: KeyRotationActionState) {
        match action {
            KeyRotationActionState::PreAnnounce { rotation_id } => {
                self.pre_announce_new_key(rotation_id).await
            }
            KeyRotationActionState::SwapDefault {
                expected_new_rotation,
            } => self.swap_default_key(expected_new_rotation),
            KeyRotationActionState::PurgeOld => {
                self.purge_old_rotation_data();
            }
            KeyRotationActionState::Wait => {}
        }
    }

    pub(crate) async fn run(&self) {
        info!("starting sphinx key rotation controller");

        let mut polling_interval = interval(self.regular_polling_interval);
        polling_interval.reset();

        let mut next_action = self.try_determine_next_action().await;
        debug!(
            "next key rotation action to take: {:?} at {}",
            next_action.typ, next_action.deadline
        );
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

            next_action = self.try_determine_next_action().await;
            debug!(
                "next key rotation action to take: {:?} at {}",
                next_action.typ, next_action.deadline
            );
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
