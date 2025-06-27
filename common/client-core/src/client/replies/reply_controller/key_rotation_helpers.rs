// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_validator_client::models::{
    EpochId, KeyRotationId, KeyRotationInfoResponse, KeyRotationState,
};
use std::time::Duration;
use time::OffsetDateTime;

pub(crate) struct ReferenceEpoch {
    pub(crate) absolute_epoch_id: EpochId,
    pub(crate) start_time: OffsetDateTime,
}

pub(crate) struct KeyRotationConfig {
    pub(crate) epoch_duration: Duration,
    pub(crate) rotation_state: KeyRotationState,
    pub(crate) reference_epoch: ReferenceEpoch,
}

impl From<KeyRotationInfoResponse> for KeyRotationConfig {
    fn from(value: KeyRotationInfoResponse) -> Self {
        KeyRotationConfig {
            epoch_duration: value.epoch_duration,
            rotation_state: value.key_rotation_state,
            reference_epoch: ReferenceEpoch {
                absolute_epoch_id: value.current_absolute_epoch_id,
                start_time: value.current_epoch_start,
            },
        }
    }
}

impl KeyRotationConfig {
    pub(crate) fn rotation_lifetime(&self) -> Duration {
        (self.rotation_state.validity_epochs + 1) * self.epoch_duration
    }

    pub(crate) fn key_rotation_id(&self, current_absolute_epoch_id: EpochId) -> KeyRotationId {
        self.rotation_state
            .key_rotation_id(current_absolute_epoch_id)
    }

    // this is called with the assumption that now is always > reference epoch start
    pub(crate) fn expected_current_epoch_id(&self, now: OffsetDateTime) -> EpochId {
        let diff_secs = (now - self.reference_epoch.start_time).as_seconds_f64();
        let epochs = (diff_secs / self.epoch_duration.as_secs_f64()).floor() as u32;

        self.reference_epoch.absolute_epoch_id + epochs
    }

    pub(crate) fn expected_time_until_next_rotation(&self) -> Option<Duration> {
        todo!()
    }

    fn initial_rotation_epoch_start(&self) -> OffsetDateTime {
        let epochs_diff =
            self.reference_epoch.absolute_epoch_id - self.rotation_state.validity_epochs;
        self.reference_epoch.start_time - epochs_diff * self.epoch_duration
    }

    pub(crate) fn key_rotation_start(&self, key_rotation_id: KeyRotationId) -> OffsetDateTime {
        let rotation_duration = self.rotation_state.validity_epochs * self.epoch_duration;
        let initial_start = self.initial_rotation_epoch_start();

        // note: key rotation starts from 0
        initial_start + rotation_duration * key_rotation_id
    }

    pub(crate) fn new_rotation_expected(&self) -> bool {
        todo!()
    }

    pub(crate) fn expected_current_key_rotation_id(&self, now: OffsetDateTime) -> KeyRotationId {
        let expected_current_epoch = self.expected_current_epoch_id(now);
        self.key_rotation_id(expected_current_epoch)
    }

    pub(crate) fn expected_current_key_rotation_start(
        &self,
        now: OffsetDateTime,
    ) -> OffsetDateTime {
        let expected_current_key_rotation_id = self.expected_current_key_rotation_id(now);
        self.key_rotation_start(expected_current_key_rotation_id)
    }
}
