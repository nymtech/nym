// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use crate::error::ContractError::EpochInProgress;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::support::helpers::is_authorized;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Storage};
use mixnet_contract_common::events::{new_advance_interval_event, new_change_rewarded_set_event};
use mixnet_contract_common::{IdentityKey, Interval};

// We've distributed the rewards to the rewarded set from the validator api before making this call (implicit order, should be solved in the future)
// We now write the new rewarded set, snapshot the mixnodes and finally reconcile all delegations and undelegations. That way the rewards for the previous
// epoch will be calculated correctly as the delegations and undelegations from the previous epoch will only take effect in the next (current) one.
pub fn try_write_rewarded_set(
    deps: DepsMut<'_>,
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

    // We don't want more then we need, less should be fine, as we could have less nodes bonded overall
    if active_set_size > state.params.mixnode_active_set_size {
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

    let block_height = env.block.height;

    if let Some(last_update) = storage::CURRENT_REWARDED_SET_HEIGHT.may_load(deps.storage)? {
        if last_update + crate::constants::REWARDED_SET_REFRESH_BLOCKS > block_height {
            return Err(ContractError::TooFrequentRewardedSetUpdate {
                last_update,
                minimum_delay: crate::constants::REWARDED_SET_REFRESH_BLOCKS,
                current_height: block_height,
            });
        }
    }
    let num_nodes = rewarded_set.len();

    storage::save_rewarded_set(deps.storage, block_height, active_set_size, rewarded_set)?;
    storage::CURRENT_REWARDED_SET_HEIGHT.save(deps.storage, &block_height)?;

    Ok(Response::new().add_event(new_change_rewarded_set_event(
        state.params.mixnode_active_set_size,
        state.params.mixnode_rewarded_set_size,
        num_nodes as u32,
    )))
}

pub fn try_init_epoch(
    info: MessageInfo,
    storage: &mut dyn Storage,
    env: Env,
) -> Result<Response, ContractError> {
    is_authorized(info.sender.as_str().to_string(), storage)?;

    init_epoch(storage, env)?;

    Ok(Response::default())
}

pub fn init_epoch(storage: &mut dyn Storage, env: Env) -> Result<Interval, ContractError> {
    let epoch = Interval::init_epoch(env);
    storage::save_epoch(storage, &epoch)?;
    Ok(epoch)
}

pub fn try_advance_epoch(
    env: Env,
    storage: &mut dyn Storage,
    sender: String,
) -> Result<Response, ContractError> {
    // in theory, we could have just changed the state and relied on its reversal upon failed
    // execution, but better safe than sorry and do not modify the state at all unless we know
    // all checks have succeeded.

    // Only rewarding validator can attempt to advance epoch

    is_authorized(sender, storage)?;

    let current_epoch = storage::current_epoch(storage)?;
    if current_epoch.is_over(env.clone()) {
        let next_epoch = current_epoch.next_on_chain(env);

        storage::save_epoch(storage, &next_epoch)?;
        storage::save_epoch_reward_params(next_epoch.id(), storage)?;

        return Ok(Response::new().add_event(new_advance_interval_event(next_epoch)));
    }
    Err(EpochInProgress {
        current_block_time: env.block.time.seconds(),
        epoch_start: current_epoch.start_unix_timestamp(),
        epoch_end: current_epoch.end_unix_timestamp(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Timestamp;
    use mixnet_contract_common::RewardedSetNodeStatus;
    use mixnet_params_storage::rewarding_validator_address;

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
                minimum_delay: crate::constants::REWARDED_SET_REFRESH_BLOCKS,
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
        env.block.height = last_update + crate::constants::REWARDED_SET_REFRESH_BLOCKS;
        let expected_response = Response::new().add_event(new_change_rewarded_set_event(
            current_state.params.mixnode_active_set_size,
            current_state.params.mixnode_rewarded_set_size,
            full_rewarded_set.len() as u32,
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
        let sender = rewarding_validator_address(&deps.storage).unwrap();

        let _current_epoch = init_epoch(&mut deps.storage, env.clone()).unwrap();

        // Works as its after the current epoch
        env.block.time = Timestamp::from_seconds(1641081600);
        assert!(try_advance_epoch(env.clone(), deps.as_mut().storage, sender.clone()).is_ok());

        let current_epoch = crate::interval::storage::current_epoch(&mut deps.storage).unwrap();

        // same if the current blocktime is set to BEFORE the first interval has even begun
        // (say we decided to set the first interval to be some time in the future at initialisation)
        env.block.time = Timestamp::from_seconds(1609459200);
        assert_eq!(
            Err(ContractError::EpochInProgress {
                current_block_time: 1609459200,
                epoch_start: current_epoch.start_unix_timestamp(),
                epoch_end: current_epoch.end_unix_timestamp()
            }),
            try_advance_epoch(env.clone(), deps.as_mut().storage, sender.clone(),)
        );

        // works otherwise

        // interval that has just finished
        env.block.time =
            Timestamp::from_seconds(current_epoch.start_unix_timestamp() as u64 + 10000);
        let expected_new_epoch = current_epoch.next_on_chain(env.clone());
        let expected_response =
            Response::new().add_event(new_advance_interval_event(expected_new_epoch));
        assert_eq!(
            Ok(expected_response),
            try_advance_epoch(env.clone(), deps.as_mut().storage, sender)
        );

        // interval way back in the past (i.e. 'somebody' failed to advance it for a long time)
        env.block.time = Timestamp::from_seconds(1672531200);
        storage::save_epoch(deps.as_mut().storage, &current_epoch).unwrap();
        let expected_new_epoch = current_epoch.next_on_chain(env.clone());
        let expected_response =
            Response::new().add_event(new_advance_interval_event(expected_new_epoch));
        let sender = rewarding_validator_address(&deps.storage).unwrap();
        assert_eq!(
            Ok(expected_response),
            try_advance_epoch(env.clone(), deps.as_mut().storage, sender)
        );
    }
}
