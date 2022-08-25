// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{INITIAL_GATEWAY_PLEDGE_AMOUNT, INITIAL_MIXNODE_PLEDGE_AMOUNT};
use crate::delegations;
use crate::interval::storage as interval_storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::storage as mixnode_storage;
use crate::mixnodes::storage::{assign_layer, next_mixnode_id_counter};
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{
    coin, entry_point, to_binary, Addr, Coin, Deps, DepsMut, Env, MessageInfo, QueryResponse,
    Response,
};
use cw_storage_plus::Item;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{
    ContractState, ContractStateParams, Delegation, ExecuteMsg, InstantiateMsg, Interval,
    MigrateMsg, MixNode, MixNodeBond, MixNodeCostParams, MixNodeRewarding, Percent, QueryMsg,
};

// To be removed once entire contract is unlocked
const V1_MIXNET_CONTRACT: Item<'_, String> = Item::new("v1_mixnet_contract");

fn default_initial_state(
    owner: Addr,
    rewarding_validator_address: Addr,
    rewarding_denom: String,
    vesting_contract_address: Addr,
) -> ContractState {
    ContractState {
        owner,
        rewarding_validator_address,
        vesting_contract_address,
        rewarding_denom: rewarding_denom.clone(),
        params: ContractStateParams {
            minimum_mixnode_delegation: None,
            minimum_mixnode_pledge: Coin {
                denom: rewarding_denom.clone(),
                amount: INITIAL_MIXNODE_PLEDGE_AMOUNT,
            },
            minimum_gateway_pledge: Coin {
                denom: rewarding_denom,
                amount: INITIAL_GATEWAY_PLEDGE_AMOUNT,
            },
        },
    }
}

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, MixnetContractError> {
    let rewarding_validator_address = deps.api.addr_validate(&msg.rewarding_validator_address)?;
    let vesting_contract_address = deps.api.addr_validate(&msg.vesting_contract_address)?;
    let state = default_initial_state(
        info.sender,
        rewarding_validator_address,
        msg.rewarding_denom,
        vesting_contract_address,
    );
    let starting_interval =
        Interval::init_interval(msg.epochs_in_interval, msg.epoch_duration, &env);
    let reward_params = msg
        .initial_rewarding_params
        .into_rewarding_params(msg.epochs_in_interval);

    interval_storage::initialise_storage(deps.storage, starting_interval)?;
    mixnet_params_storage::initialise_storage(deps.storage, state)?;
    mixnode_storage::initialise_storage(deps.storage)?;
    rewards_storage::initialise_storage(deps.storage, reward_params)?;

    V1_MIXNET_CONTRACT.save(deps.storage, &msg.v1_mixnet_contract_address)?;

    Ok(Response::default())
}

/// Handle an incoming message
#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, MixnetContractError> {
    // only the old mixnet contract is allowed to request migration-specific commands here
    if info.sender != V1_MIXNET_CONTRACT.load(deps.storage)? {
        return Err(MixnetContractError::Unauthorized);
    }

    match msg {
        ExecuteMsg::SaveOperator {
            host,
            mix_port,
            verloc_port,
            http_api_port,
            sphinx_key,
            identity_key,
            version,
            pledge_amount,
            owner,
            block_height,
            profit_margin_percent,
            proxy,
        } => {
            let mixnode = MixNode {
                host,
                mix_port,
                verloc_port,
                http_api_port,
                sphinx_key,
                identity_key,
                version,
            };
            let layer = assign_layer(deps.storage)?;

            let cost_params = MixNodeCostParams {
                // this value must be valid since we got it from the v1 contract which we trust
                profit_margin_percent: Percent::from_percentage_value(profit_margin_percent as u64)
                    .unwrap(),
                interval_operating_cost: coin(40_000_000, &pledge_amount.denom),
            };
            let node_id = next_mixnode_id_counter(deps.storage)?;
            let current_epoch =
                interval_storage::current_interval(deps.storage)?.current_epoch_absolute_id();
            let mixnode_rewarding =
                MixNodeRewarding::initialise_new(cost_params, &pledge_amount, current_epoch);
            let mixnode_bond = MixNodeBond::new(
                node_id,
                owner,
                pledge_amount,
                layer,
                mixnode,
                proxy,
                block_height,
            );

            mixnode_storage::mixnode_bonds().save(deps.storage, node_id, &mixnode_bond)?;
            rewards_storage::MIXNODE_REWARDING.save(deps.storage, node_id, &mixnode_rewarding)?;

            Ok(Response::new())
        }
        ExecuteMsg::SaveDelegation {
            owner,
            mix_id,
            amount,
            block_height,
            proxy,
        } => {
            let mut mix_rewarding =
                rewards_storage::MIXNODE_REWARDING.load(deps.storage, mix_id)?;
            mix_rewarding.add_base_delegation(amount.amount);
            rewards_storage::MIXNODE_REWARDING.save(deps.storage, mix_id, &mix_rewarding)?;

            let delegation = Delegation::new(
                owner,
                mix_id,
                mix_rewarding.total_unit_reward,
                amount,
                block_height,
                proxy,
            );

            let storage_key = delegation.storage_key();
            delegations::storage::delegations().save(deps.storage, storage_key, &delegation)?;
            Ok(Response::new())
        }
        _ => Err(MixnetContractError::MigrationInProgress),
    }
}

#[entry_point]
pub fn query(
    deps: Deps<'_>,
    env: Env,
    msg: QueryMsg,
) -> Result<QueryResponse, MixnetContractError> {
    let query_res = match msg {
        QueryMsg::GetContractVersion {} => {
            to_binary(&crate::mixnet_contract_settings::queries::query_contract_version())
        }
        QueryMsg::GetStateParams {} => to_binary(
            &crate::mixnet_contract_settings::queries::query_contract_settings_params(deps)?,
        ),
        QueryMsg::GetRewardingValidatorAddress {} => to_binary(
            &crate::mixnet_contract_settings::queries::query_rewarding_validator_address(deps)?,
        ),
        QueryMsg::GetState {} => {
            to_binary(&crate::mixnet_contract_settings::queries::query_contract_state(deps)?)
        }
        QueryMsg::GetRewardingParams {} => {
            to_binary(&crate::rewards::queries::query_rewarding_params(deps)?)
        }
        QueryMsg::GetCurrentIntervalDetails {} => to_binary(
            &crate::interval::queries::query_current_interval_details(deps, env)?,
        ),
        QueryMsg::GetRewardedSet { limit, start_after } => to_binary(
            &crate::interval::queries::query_rewarded_set_paged(deps, start_after, limit)?,
        ),

        // mixnode-related:
        QueryMsg::GetMixNodeBonds { start_after, limit } => to_binary(
            &crate::mixnodes::queries::query_mixnode_bonds_paged(deps, start_after, limit)?,
        ),
        QueryMsg::GetMixNodesDetailed { start_after, limit } => to_binary(
            &crate::mixnodes::queries::query_mixnodes_details_paged(deps, start_after, limit)?,
        ),
        QueryMsg::GetUnbondedMixNodes { limit, start_after } => to_binary(
            &crate::mixnodes::queries::query_unbonded_mixnodes_paged(deps, start_after, limit)?,
        ),
        QueryMsg::GetUnbondedMixNodesByOwner {
            owner,
            limit,
            start_after,
        } => to_binary(
            &crate::mixnodes::queries::query_unbonded_mixnodes_by_owner_paged(
                deps,
                owner,
                start_after,
                limit,
            )?,
        ),
        QueryMsg::GetUnbondedMixNodesByIdentityKey {
            identity_key,
            limit,
            start_after,
        } => to_binary(
            &crate::mixnodes::queries::query_unbonded_mixnodes_by_identity_paged(
                deps,
                identity_key,
                start_after,
                limit,
            )?,
        ),
        QueryMsg::GetOwnedMixnode { address } => to_binary(
            &crate::mixnodes::queries::query_owned_mixnode(deps, address)?,
        ),
        QueryMsg::GetMixnodeDetails { mix_id } => to_binary(
            &crate::mixnodes::queries::query_mixnode_details(deps, mix_id)?,
        ),
        QueryMsg::GetMixnodeRewardingDetails { mix_id } => to_binary(
            &crate::mixnodes::queries::query_mixnode_rewarding_details(deps, mix_id)?,
        ),
        QueryMsg::GetStakeSaturation { mix_id } => to_binary(
            &crate::mixnodes::queries::query_stake_saturation(deps, mix_id)?,
        ),
        QueryMsg::GetUnbondedMixNodeInformation { mix_id } => to_binary(
            &crate::mixnodes::queries::query_unbonded_mixnode(deps, mix_id)?,
        ),
        QueryMsg::GetBondedMixnodeDetailsByIdentity { mix_identity } => to_binary(
            &crate::mixnodes::queries::query_mixnode_details_by_identity(deps, mix_identity)?,
        ),
        QueryMsg::GetLayerDistribution {} => {
            to_binary(&crate::mixnodes::queries::query_layer_distribution(deps)?)
        }

        // gateway-related:
        QueryMsg::GetGateways { limit, start_after } => to_binary(
            &crate::gateways::queries::query_gateways_paged(deps, start_after, limit)?,
        ),
        QueryMsg::GetGatewayBond { identity } => to_binary(
            &crate::gateways::queries::query_gateway_bond(deps, identity)?,
        ),
        QueryMsg::GetOwnedGateway { address } => to_binary(
            &crate::gateways::queries::query_owned_gateway(deps, address)?,
        ),

        // delegation-related:
        QueryMsg::GetMixnodeDelegations {
            mix_id,
            start_after,
            limit,
        } => to_binary(
            &crate::delegations::queries::query_mixnode_delegations_paged(
                deps,
                mix_id,
                start_after,
                limit,
            )?,
        ),
        QueryMsg::GetDelegatorDelegations {
            delegator,
            start_after,
            limit,
        } => to_binary(
            &crate::delegations::queries::query_delegator_delegations_paged(
                deps,
                delegator,
                start_after,
                limit,
            )?,
        ),
        QueryMsg::GetDelegationDetails {
            mix_id,
            delegator,
            proxy,
        } => to_binary(&crate::delegations::queries::query_mixnode_delegation(
            deps, mix_id, delegator, proxy,
        )?),
        QueryMsg::GetAllDelegations { start_after, limit } => to_binary(
            &crate::delegations::queries::query_all_delegations_paged(deps, start_after, limit)?,
        ),

        // rewards related
        QueryMsg::GetPendingOperatorReward { address } => to_binary(
            &crate::rewards::queries::query_pending_operator_reward(deps, address)?,
        ),
        QueryMsg::GetPendingMixNodeOperatorReward { mix_id } => to_binary(
            &crate::rewards::queries::query_pending_mixnode_operator_reward(deps, mix_id)?,
        ),
        QueryMsg::GetPendingDelegatorReward {
            address,
            mix_id,
            proxy,
        } => to_binary(&crate::rewards::queries::query_pending_delegator_reward(
            deps, address, mix_id, proxy,
        )?),
        QueryMsg::GetEstimatedCurrentEpochOperatorReward {
            mix_id,
            estimated_performance,
        } => to_binary(
            &crate::rewards::queries::query_estimated_current_epoch_operator_reward(
                deps,
                mix_id,
                estimated_performance,
            )?,
        ),
        QueryMsg::GetEstimatedCurrentEpochDelegatorReward {
            address,
            mix_id,
            proxy,
            estimated_performance,
        } => to_binary(
            &crate::rewards::queries::query_estimated_current_epoch_delegator_reward(
                deps,
                address,
                mix_id,
                proxy,
                estimated_performance,
            )?,
        ),

        // interval-related
        QueryMsg::GetPendingEpochEvents { limit, start_after } => {
            to_binary(&crate::interval::queries::query_pending_epoch_events_paged(
                deps,
                env,
                start_after,
                limit,
            )?)
        }
        QueryMsg::GetPendingIntervalEvents { limit, start_after } => to_binary(
            &crate::interval::queries::query_pending_interval_events_paged(
                deps,
                env,
                start_after,
                limit,
            )?,
        ),
    };

    Ok(query_res?)
}

#[entry_point]
pub fn migrate(
    _deps: DepsMut<'_>,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, MixnetContractError> {
    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::Decimal;
    use mixnet_contract_common::reward_params::{IntervalRewardParams, RewardingParams};
    use mixnet_contract_common::{InitialRewardingParams, Percent};
    use std::time::Duration;

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let init_msg = InstantiateMsg {
            rewarding_validator_address: "foomp123".to_string(),
            vesting_contract_address: "bar456".to_string(),
            v1_mixnet_contract_address: "whatever".to_string(),
            rewarding_denom: "uatom".to_string(),
            epochs_in_interval: 1234,
            epoch_duration: Duration::from_secs(4321),
            initial_rewarding_params: InitialRewardingParams {
                initial_reward_pool: Decimal::from_atomics(100_000_000_000_000u128, 0).unwrap(),
                initial_staking_supply: Decimal::from_atomics(123_456_000_000_000u128, 0).unwrap(),
                sybil_resistance: Percent::from_percentage_value(23).unwrap(),
                active_set_work_factor: Decimal::from_atomics(10u32, 0).unwrap(),
                interval_pool_emission: Percent::from_percentage_value(1).unwrap(),
                rewarded_set_size: 543,
                active_set_size: 123,
            },
        };

        let sender = mock_info("sender", &[]);
        let res = instantiate(deps.as_mut(), env, sender, init_msg);
        assert!(res.is_ok());

        let expected_state = ContractState {
            owner: Addr::unchecked("sender"),
            rewarding_validator_address: Addr::unchecked("foomp123"),
            vesting_contract_address: Addr::unchecked("bar456"),
            rewarding_denom: "uatom".into(),
            params: ContractStateParams {
                minimum_mixnode_delegation: None,
                minimum_mixnode_pledge: Coin {
                    denom: "uatom".into(),
                    amount: INITIAL_MIXNODE_PLEDGE_AMOUNT,
                },
                minimum_gateway_pledge: Coin {
                    denom: "uatom".into(),
                    amount: INITIAL_GATEWAY_PLEDGE_AMOUNT,
                },
            },
        };

        let expected_epoch_reward_budget =
            Decimal::from_ratio(100_000_000_000_000u128, 1234u32) * Decimal::percent(1);
        let expected_stake_saturation_point = Decimal::from_ratio(123_456_000_000_000u128, 543u32);

        let expected_rewarding_params = RewardingParams {
            interval: IntervalRewardParams {
                reward_pool: Decimal::from_atomics(100_000_000_000_000u128, 0).unwrap(),
                staking_supply: Decimal::from_atomics(123_456_000_000_000u128, 0).unwrap(),
                epoch_reward_budget: expected_epoch_reward_budget,
                stake_saturation_point: expected_stake_saturation_point,
                sybil_resistance: Percent::from_percentage_value(23).unwrap(),
                active_set_work_factor: Decimal::from_atomics(10u32, 0).unwrap(),
                interval_pool_emission: Percent::from_percentage_value(1).unwrap(),
            },
            rewarded_set_size: 543,
            active_set_size: 123,
        };

        let state = mixnet_params_storage::CONTRACT_STATE
            .load(deps.as_ref().storage)
            .unwrap();
        assert_eq!(state, expected_state);

        let rewarding_params = rewards_storage::REWARDING_PARAMS
            .load(deps.as_ref().storage)
            .unwrap();
        assert_eq!(rewarding_params, expected_rewarding_params);

        let interval = interval_storage::current_interval(deps.as_ref().storage).unwrap();
        assert_eq!(interval.epochs_in_interval(), 1234);
        assert_eq!(interval.epoch_length(), Duration::from_secs(4321));
        assert_eq!(interval.current_interval_id(), 0);
        assert_eq!(interval.current_epoch_id(), 0);
    }
}
