// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_topology::NymTopologyMetadata;
use nym_validator_client::models::{
    EpochId, KeyRotationId, KeyRotationInfoResponse, KeyRotationState,
};
use std::time::Duration;
use time::OffsetDateTime;

#[derive(Clone, Copy)]
pub(crate) enum SurbRefreshState {
    WaitingForNextRotation { last_known: KeyRotationId },
    ScheduledForNextInvocation,
}

#[derive(Clone, Copy)]
pub(crate) struct ReferenceEpoch {
    pub(crate) absolute_epoch_id: EpochId,
    pub(crate) start_time: OffsetDateTime,
}

#[derive(Clone, Copy)]
pub(crate) struct KeyRotationConfig {
    pub(crate) epoch_duration: Duration,
    pub(crate) rotation_state: KeyRotationState,
    pub(crate) reference_epoch: ReferenceEpoch,
}

impl From<KeyRotationInfoResponse> for KeyRotationConfig {
    fn from(value: KeyRotationInfoResponse) -> Self {
        KeyRotationConfig {
            epoch_duration: value.details.epoch_duration,
            rotation_state: value.details.key_rotation_state,
            reference_epoch: ReferenceEpoch {
                absolute_epoch_id: value.details.current_absolute_epoch_id,
                start_time: value.details.current_epoch_start,
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

    fn initial_rotation_epoch_start(&self) -> OffsetDateTime {
        let epochs_diff = self
            .reference_epoch
            .absolute_epoch_id
            .saturating_sub(self.rotation_state.initial_epoch_id);

        self.reference_epoch.start_time - epochs_diff * self.epoch_duration
    }

    pub(crate) fn key_rotation_start(&self, key_rotation_id: KeyRotationId) -> OffsetDateTime {
        let rotation_duration = self.rotation_state.validity_epochs * self.epoch_duration;
        let initial_start = self.initial_rotation_epoch_start();

        // note: key rotation starts from 0
        initial_start + rotation_duration * key_rotation_id
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

    pub(crate) fn epoch_stuck(&self, topology_metadata: NymTopologyMetadata) -> bool {
        // add leeway of 2mins each direction since transition is not instantaneous
        let lower_bound = topology_metadata.refreshed_at - Duration::from_secs(2);
        let upper_bound = topology_metadata.refreshed_at + Duration::from_secs(2);

        let expected_epoch_lower = self.expected_current_epoch_id(lower_bound);
        let expected_epoch_upper = self.expected_current_epoch_id(upper_bound);

        topology_metadata.absolute_epoch_id != expected_epoch_lower
            && topology_metadata.absolute_epoch_id != expected_epoch_upper
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn mock_config() -> KeyRotationConfig {
        KeyRotationConfig {
            epoch_duration: Duration::from_secs(60 * 60),
            rotation_state: KeyRotationState {
                validity_epochs: 10,
                initial_epoch_id: 80,
            },
            reference_epoch: ReferenceEpoch {
                absolute_epoch_id: 100,
                start_time: datetime!(2025-06-30 12:00:00+00:00),
            },
        }
    }

    #[test]
    fn expected_current_key_rotation_start() {
        // rot0: 80-89
        // rot1: 90-99
        // rot2: 100-109
        // rot3: 110-119
        // ... etc
        let cfg = mock_config();

        assert_eq!(
            cfg.initial_rotation_epoch_start(),
            datetime!(2025-06-29 16:00:00+00:00)
        );

        let fake_now = datetime!(2025-06-30 12:00:00+00:00);
        assert_eq!(cfg.expected_current_epoch_id(fake_now), 100);
        assert_eq!(cfg.expected_current_key_rotation_id(fake_now), 2);
        assert_eq!(
            cfg.expected_current_key_rotation_start(fake_now),
            datetime!(2025-06-30 12:00:00+00:00)
        );

        let fake_now = datetime!(2025-06-30 12:30:00+00:00);
        assert_eq!(cfg.expected_current_epoch_id(fake_now), 100);
        assert_eq!(cfg.expected_current_key_rotation_id(fake_now), 2);
        assert_eq!(
            cfg.expected_current_key_rotation_start(fake_now),
            datetime!(2025-06-30 12:00:00+00:00)
        );

        let fake_now = datetime!(2025-06-30 13:01:00+00:00);
        assert_eq!(cfg.expected_current_epoch_id(fake_now), 101);
        assert_eq!(cfg.expected_current_key_rotation_id(fake_now), 2);
        assert_eq!(
            cfg.expected_current_key_rotation_start(fake_now),
            datetime!(2025-06-30 12:00:00+00:00)
        );

        let fake_now = datetime!(2025-06-30 22:02:00+00:00);
        assert_eq!(cfg.expected_current_epoch_id(fake_now), 110);
        assert_eq!(cfg.expected_current_key_rotation_id(fake_now), 3);
        assert_eq!(
            cfg.expected_current_key_rotation_start(fake_now),
            datetime!(2025-06-30 22:00:00+00:00)
        );
    }
}
