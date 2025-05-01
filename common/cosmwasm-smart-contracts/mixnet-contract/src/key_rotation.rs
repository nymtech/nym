// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::EpochId;
use cosmwasm_schema::cw_serde;

pub type KeyRotationId = u32;

#[cw_serde]
pub struct KeyRotationState {
    /// Defines how long each key rotation is valid for (in terms of epochs)
    pub validity_epochs: u32,

    /// Records the initial epoch_id when the key rotation has been introduced (0 for fresh contracts).
    /// It is used for determining when rotation is meant to advance.
    pub initial_epoch_id: EpochId,
}

impl KeyRotationState {
    pub fn key_rotation_id(&self, current_epoch_id: EpochId) -> KeyRotationId {
        let diff = current_epoch_id.saturating_sub(self.initial_epoch_id);
        let full_rots = diff / self.validity_epochs;
        full_rots
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
}
