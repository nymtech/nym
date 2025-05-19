// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::EpochId;
use cosmwasm_schema::cw_serde;

pub type KeyRotationId = u32;

#[cw_serde]
#[derive(Copy)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct KeyRotationState {
    /// Defines how long each key rotation is valid for (in terms of epochs)
    pub validity_epochs: u32,

    /// Records the initial epoch_id when the key rotation has been introduced (0 for fresh contracts).
    /// It is used for determining when rotation is meant to advance.
    #[cfg_attr(feature = "utoipa", schema(value_type = u32))]
    pub initial_epoch_id: EpochId,
}

impl KeyRotationState {
    pub fn key_rotation_id(&self, current_epoch_id: EpochId) -> KeyRotationId {
        let diff = current_epoch_id.saturating_sub(self.initial_epoch_id);
        diff / self.validity_epochs
    }

    pub fn next_rotation_starting_epoch_id(&self, current_epoch_id: EpochId) -> EpochId {
        let current_rotation_id = self.key_rotation_id(current_epoch_id);

        self.initial_epoch_id + self.validity_epochs * (current_rotation_id + 1)
    }

    pub fn current_rotation_starting_epoch_id(&self, current_epoch_id: EpochId) -> EpochId {
        let current_rotation_id = self.key_rotation_id(current_epoch_id);

        self.initial_epoch_id + self.validity_epochs * current_rotation_id
    }
}

#[cw_serde]
pub struct KeyRotationIdResponse {
    pub rotation_id: KeyRotationId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_rotation_id() {
        let state = KeyRotationState {
            validity_epochs: 24,
            initial_epoch_id: 0,
        };
        assert_eq!(0, state.key_rotation_id(0));
        assert_eq!(0, state.key_rotation_id(23));
        assert_eq!(1, state.key_rotation_id(24));
        assert_eq!(1, state.key_rotation_id(47));
        assert_eq!(2, state.key_rotation_id(48));

        let state = KeyRotationState {
            validity_epochs: 12,
            initial_epoch_id: 0,
        };
        assert_eq!(0, state.key_rotation_id(0));
        assert_eq!(0, state.key_rotation_id(11));
        assert_eq!(1, state.key_rotation_id(12));
        assert_eq!(1, state.key_rotation_id(23));
        assert_eq!(2, state.key_rotation_id(24));

        let state = KeyRotationState {
            validity_epochs: 24,
            initial_epoch_id: 10000,
        };
        assert_eq!(0, state.key_rotation_id(123));
        assert_eq!(0, state.key_rotation_id(10000));
        assert_eq!(0, state.key_rotation_id(10001));
        assert_eq!(0, state.key_rotation_id(10023));
        assert_eq!(1, state.key_rotation_id(10024));
        assert_eq!(1, state.key_rotation_id(10047));
        assert_eq!(2, state.key_rotation_id(10048));
        assert_eq!(2, state.key_rotation_id(10060));
    }

    #[test]
    fn next_rotation_starting_epoch_id() {
        let state = KeyRotationState {
            validity_epochs: 24,
            initial_epoch_id: 0,
        };
        assert_eq!(24, state.next_rotation_starting_epoch_id(0));
        assert_eq!(24, state.next_rotation_starting_epoch_id(23));
        assert_eq!(48, state.next_rotation_starting_epoch_id(24));
        assert_eq!(48, state.next_rotation_starting_epoch_id(47));
        assert_eq!(72, state.next_rotation_starting_epoch_id(48));

        let state = KeyRotationState {
            validity_epochs: 12,
            initial_epoch_id: 0,
        };
        assert_eq!(12, state.next_rotation_starting_epoch_id(0));
        assert_eq!(12, state.next_rotation_starting_epoch_id(11));
        assert_eq!(24, state.next_rotation_starting_epoch_id(12));
        assert_eq!(24, state.next_rotation_starting_epoch_id(23));
        assert_eq!(36, state.next_rotation_starting_epoch_id(24));

        let state = KeyRotationState {
            validity_epochs: 24,
            initial_epoch_id: 10000,
        };
        assert_eq!(10024, state.next_rotation_starting_epoch_id(123));
        assert_eq!(10024, state.next_rotation_starting_epoch_id(10000));
        assert_eq!(10024, state.next_rotation_starting_epoch_id(10001));
        assert_eq!(10024, state.next_rotation_starting_epoch_id(10023));
        assert_eq!(10048, state.next_rotation_starting_epoch_id(10024));
        assert_eq!(10048, state.next_rotation_starting_epoch_id(10047));
        assert_eq!(10072, state.next_rotation_starting_epoch_id(10048));
        assert_eq!(10072, state.next_rotation_starting_epoch_id(10060));
    }

    #[test]
    fn current_rotation_starting_epoch_id() {
        let state = KeyRotationState {
            validity_epochs: 24,
            initial_epoch_id: 0,
        };
        assert_eq!(0, state.current_rotation_starting_epoch_id(0));
        assert_eq!(0, state.current_rotation_starting_epoch_id(23));
        assert_eq!(24, state.current_rotation_starting_epoch_id(24));
        assert_eq!(24, state.current_rotation_starting_epoch_id(47));
        assert_eq!(48, state.current_rotation_starting_epoch_id(48));

        let state = KeyRotationState {
            validity_epochs: 12,
            initial_epoch_id: 0,
        };
        assert_eq!(0, state.current_rotation_starting_epoch_id(0));
        assert_eq!(0, state.current_rotation_starting_epoch_id(11));
        assert_eq!(12, state.current_rotation_starting_epoch_id(12));
        assert_eq!(12, state.current_rotation_starting_epoch_id(23));
        assert_eq!(24, state.current_rotation_starting_epoch_id(24));

        let state = KeyRotationState {
            validity_epochs: 24,
            initial_epoch_id: 10000,
        };
        assert_eq!(10000, state.current_rotation_starting_epoch_id(123));
        assert_eq!(10000, state.current_rotation_starting_epoch_id(10000));
        assert_eq!(10000, state.current_rotation_starting_epoch_id(10001));
        assert_eq!(10000, state.current_rotation_starting_epoch_id(10023));
        assert_eq!(10024, state.current_rotation_starting_epoch_id(10024));
        assert_eq!(10024, state.current_rotation_starting_epoch_id(10047));
        assert_eq!(10048, state.current_rotation_starting_epoch_id(10048));
        assert_eq!(10048, state.current_rotation_starting_epoch_id(10060));
    }
}
