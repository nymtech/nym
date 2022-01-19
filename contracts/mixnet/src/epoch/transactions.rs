// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use crate::error::ContractError::EpochNotInProgress;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Storage};
use mixnet_contract_common::events::{new_advance_epoch_event, new_change_rewarded_set_event};
use mixnet_contract_common::IdentityKey;

pub fn try_write_rewarded_set(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rewarded_set: Vec<IdentityKey>,
    active_set_size: u32,
) -> Result<Response, ContractError> {
    let state = mixnet_params_storage::CONTRACT_STATE.load(deps.storage)?;

    // check if this is executed by the permitted validator, if not reject the transaction
    if info.sender != state.rewarding_validator_address {
        return Err(ContractError::Unauthorized);
    }

    // sanity check to make sure the sending validator is in sync with the contract state
    // (i.e. so that we'd known that top k nodes are actually expected to be active)
    if active_set_size != state.params.mixnode_active_set_size {
        return Err(ContractError::UnexpectedActiveSetSize {
            received: active_set_size,
            expected: state.params.mixnode_active_set_size,
        });
    }

    if rewarded_set.len() as u32 > state.params.mixnode_rewarded_set_size {
        return Err(ContractError::UnexpectedRewardedSetSize {
            received: rewarded_set.len() as u32,
            expected: state.params.mixnode_rewarded_set_size,
        });
    }

    let last_update = storage::CURRENT_REWARDED_SET_HEIGHT.load(deps.storage)?;
    let block_height = env.block.height;

    if last_update + crate::contract::REWARDED_SET_REFRESH_BLOCKS > block_height {
        return Err(ContractError::TooFrequentRewardedSetUpdate {
            last_update,
            minimum_delay: crate::contract::REWARDED_SET_REFRESH_BLOCKS,
            current_height: block_height,
        });
    }

    let current_epoch = storage::CURRENT_EPOCH.load(deps.storage)?.id();
    let num_nodes = rewarded_set.len();

    storage::save_rewarded_set(deps.storage, block_height, active_set_size, rewarded_set)?;
    storage::REWARDED_SET_HEIGHTS_FOR_EPOCH.save(
        deps.storage,
        (current_epoch, block_height),
        &0u8,
    )?;
    storage::CURRENT_REWARDED_SET_HEIGHT.save(deps.storage, &block_height)?;

    Ok(Response::new().add_event(new_change_rewarded_set_event(
        state.params.mixnode_active_set_size,
        state.params.mixnode_rewarded_set_size,
        num_nodes as u32,
        current_epoch,
    )))
}

pub fn try_advance_epoch(env: Env, storage: &mut dyn Storage) -> Result<Response, ContractError> {
    // in theory, we could have just changed the state and relied on its reversal upon failed
    // execution, but better safe than sorry and do not modify the state at all unless we know
    // all checks have succeeded.
    let current_epoch = storage::CURRENT_EPOCH.load(storage)?;
    let next_epoch = current_epoch.next_epoch();

    if next_epoch.start_unix_timestamp() > env.block.time.seconds() as i64 {
        // the reason for this check is as follows:
        // nobody, even trusted validators, should be able to continuously keep advancing epochs,
        // because otherwise it would be possible for them to continuously keep rewarding nodes.
        //
        // Therefore, even if "trusted" validator, responsible for rewarding, is malicious,
        // they can't send rewards more often than every `REWARDED_SET_REFRESH_BLOCKS`
        // and changing this value requires going through governance and having agreement of
        // the super-majority of the validators (by stake)
        return Err(EpochNotInProgress {
            current_block_time: env.block.time.seconds(),
            epoch_start: next_epoch.start_unix_timestamp(),
            epoch_end: next_epoch.end_unix_timestamp(),
        });
    }

    storage::CURRENT_EPOCH.save(storage, &next_epoch)?;

    Ok(Response::new().add_event(new_advance_epoch_event(next_epoch)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Timestamp;
    use mixnet_contract_common::{Epoch, RewardedSetNodeStatus};
    use std::time::Duration;
    use time::OffsetDateTime;

    #[test]
    fn writing_rewarded_set() {
        let mut env = mock_env();
        let mut deps = test_helpers::init_contract();
        let current_state = mixnet_params_storage::CONTRACT_STATE
            .load(deps.as_mut().storage)
            .unwrap();
        let authorised_sender = mock_info(current_state.rewarding_validator_address.as_str(), &[]);
        let full_rewarded_set = (0..current_state.params.mixnode_rewarded_set_size)
            .map(|i| format!("identity{:04}", i))
            .collect::<Vec<_>>();
        let last_update = 123;
        storage::CURRENT_REWARDED_SET_HEIGHT
            .save(deps.as_mut().storage, &last_update)
            .unwrap();

        // can only be performed by the permitted validator
        let dummy_sender = mock_info("dummy_sender", &[]);
        assert_eq!(
            Err(ContractError::Unauthorized),
            try_write_rewarded_set(
                deps.as_mut(),
                env.clone(),
                dummy_sender,
                full_rewarded_set.clone(),
                current_state.params.mixnode_active_set_size
            )
        );

        // the sender must use the same active set size as the one defined in the contract
        assert_eq!(
            Err(ContractError::UnexpectedActiveSetSize {
                received: 123,
                expected: current_state.params.mixnode_active_set_size
            }),
            try_write_rewarded_set(
                deps.as_mut(),
                env.clone(),
                authorised_sender.clone(),
                full_rewarded_set.clone(),
                123
            )
        );

        // the sender cannot provide more nodes than the rewarded set size
        let mut bigger_set = full_rewarded_set.clone();
        bigger_set.push("another_node".to_string());
        assert_eq!(
            Err(ContractError::UnexpectedRewardedSetSize {
                received: current_state.params.mixnode_rewarded_set_size + 1,
                expected: current_state.params.mixnode_rewarded_set_size
            }),
            try_write_rewarded_set(
                deps.as_mut(),
                env.clone(),
                authorised_sender.clone(),
                bigger_set,
                current_state.params.mixnode_active_set_size
            )
        );

        // cannot be performed too soon after a previous update
        env.block.height = last_update + 1;
        assert_eq!(
            Err(ContractError::TooFrequentRewardedSetUpdate {
                last_update,
                minimum_delay: crate::contract::REWARDED_SET_REFRESH_BLOCKS,
                current_height: last_update + 1,
            }),
            try_write_rewarded_set(
                deps.as_mut(),
                env.clone(),
                authorised_sender.clone(),
                full_rewarded_set.clone(),
                current_state.params.mixnode_active_set_size
            )
        );

        // after successful rewarded set write, all internal storage structures are updated appropriately
        env.block.height = last_update + crate::contract::REWARDED_SET_REFRESH_BLOCKS;
        let expected_response = Response::new().add_event(new_change_rewarded_set_event(
            current_state.params.mixnode_active_set_size,
            current_state.params.mixnode_rewarded_set_size,
            full_rewarded_set.len() as u32,
            0,
        ));

        assert_eq!(
            Ok(expected_response),
            try_write_rewarded_set(
                deps.as_mut(),
                env.clone(),
                authorised_sender,
                full_rewarded_set.clone(),
                current_state.params.mixnode_active_set_size
            )
        );

        for (i, rewarded_node) in full_rewarded_set.into_iter().enumerate() {
            if (i as u32) < current_state.params.mixnode_active_set_size {
                assert_eq!(
                    RewardedSetNodeStatus::Active,
                    storage::REWARDED_SET
                        .load(deps.as_ref().storage, (env.block.height, rewarded_node))
                        .unwrap()
                )
            } else {
                assert_eq!(
                    RewardedSetNodeStatus::Standby,
                    storage::REWARDED_SET
                        .load(deps.as_ref().storage, (env.block.height, rewarded_node))
                        .unwrap()
                )
            }
        }
        assert!(storage::REWARDED_SET_HEIGHTS_FOR_EPOCH
            .has(deps.as_ref().storage, (0, env.block.height)));
        assert_eq!(
            env.block.height,
            storage::CURRENT_REWARDED_SET_HEIGHT
                .load(deps.as_ref().storage)
                .unwrap()
        );
    }

    #[test]
    fn advancing_epoch() {
        let mut env = mock_env();
        let mut deps = test_helpers::init_contract();

        // 1609459200 = 2021-01-01
        // 1640995200 = 2022-01-01
        // 1641081600 = 2022-01-02
        // 1643673600 = 2022-02-01
        // 1672531200 = 2023-01-01

        let current_epoch = Epoch::new(
            0,
            OffsetDateTime::from_unix_timestamp(1640995200).unwrap(),
            Duration::from_secs(60 * 60 * 720),
        );
        let next_epoch = current_epoch.next_epoch();
        storage::CURRENT_EPOCH
            .save(deps.as_mut().storage, &current_epoch)
            .unwrap();

        // fails if the current epoch hasn't finished yet i.e. the new epoch hasn't begun
        env.block.time = Timestamp::from_seconds(1641081600);
        assert_eq!(
            Err(ContractError::EpochNotInProgress {
                current_block_time: 1641081600,
                epoch_start: next_epoch.start_unix_timestamp(),
                epoch_end: next_epoch.end_unix_timestamp()
            }),
            try_advance_epoch(env.clone(), deps.as_mut().storage)
        );

        // same if the current blocktime is set to BEFORE the first epoch has even begun
        // (say we decided to set the first epoch to be some time in the future at initialisation)
        env.block.time = Timestamp::from_seconds(1609459200);
        assert_eq!(
            Err(ContractError::EpochNotInProgress {
                current_block_time: 1609459200,
                epoch_start: next_epoch.start_unix_timestamp(),
                epoch_end: next_epoch.end_unix_timestamp()
            }),
            try_advance_epoch(env.clone(), deps.as_mut().storage)
        );

        // works otherwise

        // epoch that has just finished
        env.block.time = Timestamp::from_seconds(next_epoch.start_unix_timestamp() as u64 + 10000);
        let expected_new_epoch = current_epoch.next_epoch();
        let expected_response =
            Response::new().add_event(new_advance_epoch_event(expected_new_epoch));
        assert_eq!(
            Ok(expected_response),
            try_advance_epoch(env.clone(), deps.as_mut().storage)
        );

        // epoch way back in the past (i.e. 'somebody' failed to advance it for a long time)
        env.block.time = Timestamp::from_seconds(1672531200);
        storage::CURRENT_EPOCH
            .save(deps.as_mut().storage, &current_epoch)
            .unwrap();
        let expected_new_epoch = current_epoch.next_epoch();
        let expected_response =
            Response::new().add_event(new_advance_epoch_event(expected_new_epoch));
        assert_eq!(
            Ok(expected_response),
            try_advance_epoch(env.clone(), deps.as_mut().storage)
        );
    }
}
