// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::{Item, Map, SnapshotItem, Strategy};
use nym_coconut_dkg_common::types::{Epoch, EpochId};

#[deprecated]
// leave old values in storage for backwards compatibility, but make sure everything in the contract
// uses the new reference
pub(crate) const CURRENT_EPOCH: Item<Epoch> = Item::new("current_epoch");
pub const HISTORICAL_EPOCH: SnapshotItem<Epoch> = SnapshotItem::new(
    "historical_epoch",
    "historical_epoch__checkpoints",
    "historical_epoch__changelog",
    Strategy::EveryBlock,
);

pub const THRESHOLD: Item<u64> = Item::new("threshold");

pub const EPOCH_THRESHOLDS: Map<EpochId, u64> = Map::new("epoch_thresholds");

#[allow(deprecated)]
pub fn save_epoch(storage: &mut dyn Storage, height: u64, epoch: &Epoch) -> StdResult<()> {
    CURRENT_EPOCH.save(storage, epoch)?;
    HISTORICAL_EPOCH.save(storage, epoch, height)
}

#[allow(deprecated)]
pub fn load_current_epoch(storage: &dyn Storage) -> StdResult<Epoch> {
    #[cfg(debug_assertions)]
    {
        let current = CURRENT_EPOCH.load(storage);
        let historical = HISTORICAL_EPOCH.load(storage);
        debug_assert_eq!(current, historical);
    }
    HISTORICAL_EPOCH.load(storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epoch_state::transactions::{try_advance_epoch_state, try_initiate_dkg};
    use crate::support::tests::helpers::{init_contract, ADMIN_ADDRESS};
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{Addr, Env};
    use nym_coconut_dkg_common::types::EpochState;
    use std::ops::{Deref, DerefMut};

    #[test]
    fn full_dkg_correctly_updates_historical_epoch() -> anyhow::Result<()> {
        struct EnvWrapper {
            env: Env,
        }

        impl EnvWrapper {
            fn next_block(&mut self) {
                self.env.block.height += 1;
                self.env.block.time = self.env.block.time.plus_seconds(5);
            }

            fn height(&self) -> u64 {
                self.block.height
            }
        }

        impl Deref for EnvWrapper {
            type Target = Env;
            fn deref(&self) -> &Self::Target {
                &self.env
            }
        }

        impl DerefMut for EnvWrapper {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.env
            }
        }

        let mut empty_deps = mock_dependencies();

        // before contract is initialised, there's nothing saved
        assert!(HISTORICAL_EPOCH
            .may_load(empty_deps.as_mut().storage)?
            .is_none());

        let mut deps = init_contract();
        let mut env = EnvWrapper { env: mock_env() };

        let init_height = env.height();
        // after init it has initial state
        assert_eq!(HISTORICAL_EPOCH.load(deps.as_mut().storage)?.epoch_id, 0);
        assert_eq!(
            HISTORICAL_EPOCH.load(deps.as_mut().storage)?.state,
            EpochState::WaitingInitialisation
        );

        env.next_block();
        let pub_key_submission_height = env.height();
        try_initiate_dkg(
            deps.as_mut(),
            (*env).clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        )?;
        assert_eq!(
            HISTORICAL_EPOCH.load(deps.as_mut().storage)?.state,
            EpochState::PublicKeySubmission { resharing: false }
        );

        env.block.time = env.block.time.plus_seconds(100000);
        env.next_block();
        let dealing_exchange_height = env.height();
        try_advance_epoch_state(deps.as_mut(), (*env).clone())?;
        assert_eq!(
            HISTORICAL_EPOCH.load(deps.as_mut().storage)?.state,
            EpochState::DealingExchange { resharing: false }
        );

        env.block.time = env.block.time.plus_seconds(100000);
        env.next_block();
        let verification_key_submission_height = env.height();
        try_advance_epoch_state(deps.as_mut(), (*env).clone())?;
        assert_eq!(
            HISTORICAL_EPOCH.load(deps.as_mut().storage)?.state,
            EpochState::VerificationKeySubmission { resharing: false }
        );

        env.block.time = env.block.time.plus_seconds(100000);
        env.next_block();
        let verification_key_validation_height = env.height();
        try_advance_epoch_state(deps.as_mut(), (*env).clone())?;
        assert_eq!(
            HISTORICAL_EPOCH.load(deps.as_mut().storage)?.state,
            EpochState::VerificationKeyValidation { resharing: false }
        );

        env.block.time = env.block.time.plus_seconds(100000);
        env.next_block();
        let verification_key_finalization_height = env.height();
        try_advance_epoch_state(deps.as_mut(), (*env).clone())?;
        assert_eq!(
            HISTORICAL_EPOCH.load(deps.as_mut().storage)?.state,
            EpochState::VerificationKeyFinalization { resharing: false }
        );

        env.block.time = env.block.time.plus_seconds(100000);
        env.next_block();
        let in_progress_height = env.height();
        try_advance_epoch_state(deps.as_mut(), (*env).clone())?;
        assert_eq!(
            HISTORICAL_EPOCH.load(deps.as_mut().storage)?.state,
            EpochState::InProgress {}
        );

        // check old data
        assert!(HISTORICAL_EPOCH
            .may_load_at_height(deps.as_mut().storage, init_height - 1)?
            .is_none());
        assert_eq!(
            HISTORICAL_EPOCH
                .may_load_at_height(deps.as_mut().storage, init_height + 1)?
                .unwrap()
                .state,
            EpochState::WaitingInitialisation
        );
        assert_eq!(
            HISTORICAL_EPOCH
                .may_load_at_height(deps.as_mut().storage, pub_key_submission_height + 1)?
                .unwrap()
                .state,
            EpochState::PublicKeySubmission { resharing: false }
        );

        assert_eq!(
            HISTORICAL_EPOCH
                .may_load_at_height(deps.as_mut().storage, dealing_exchange_height + 1)?
                .unwrap()
                .state,
            EpochState::DealingExchange { resharing: false }
        );

        assert_eq!(
            HISTORICAL_EPOCH
                .may_load_at_height(
                    deps.as_mut().storage,
                    verification_key_submission_height + 1
                )?
                .unwrap()
                .state,
            EpochState::VerificationKeySubmission { resharing: false }
        );

        assert_eq!(
            HISTORICAL_EPOCH
                .may_load_at_height(
                    deps.as_mut().storage,
                    verification_key_validation_height + 1
                )?
                .unwrap()
                .state,
            EpochState::VerificationKeyValidation { resharing: false }
        );

        assert_eq!(
            HISTORICAL_EPOCH
                .may_load_at_height(
                    deps.as_mut().storage,
                    verification_key_finalization_height + 1
                )?
                .unwrap()
                .state,
            EpochState::VerificationKeyFinalization { resharing: false }
        );

        assert_eq!(
            HISTORICAL_EPOCH
                .may_load_at_height(deps.as_mut().storage, in_progress_height + 1)?
                .unwrap()
                .state,
            EpochState::InProgress
        );

        Ok(())
    }
}
