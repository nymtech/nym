// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{INITIAL_GATEWAY_PLEDGE_AMOUNT, INITIAL_MIXNODE_PLEDGE_AMOUNT};
use crate::interval::storage as interval_storage;
use crate::mixnet_contract_settings::models::ContractState;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::storage as mixnode_storage;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Coin, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
    Uint128,
};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{
    ContractStateParams, ExecuteMsg, InstantiateMsg, Interval, MigrateMsg, QueryMsg,
};

fn default_initial_state(
    owner: Addr,
    rewarding_validator_address: Addr,
    rewarding_denom: String,
    vesting_contract_address: Addr,
) -> ContractState {
    ContractState {
        owner,
        rewarding_validator_address,
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
            vesting_contract_address,
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

    // TODO: perhaps for each storage create a function like "initialise storage"
    interval_storage::save_interval(deps.storage, &starting_interval)?;
    interval_storage::LAST_PROCESSED_EPOCH_EVENT.save(deps.storage, &0)?;
    interval_storage::LAST_PROCESSED_INTERVAL_EVENT.save(deps.storage, &0)?;
    mixnet_params_storage::CONTRACT_STATE.save(deps.storage, &state)?;
    mixnode_storage::LAYERS.save(deps.storage, &Default::default())?;
    // rewards_storage::REWARD_POOL.save(deps.storage, &Uint128::new(INITIAL_REWARD_POOL))?;

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
    match msg {
        // state/sys-params-related
        ExecuteMsg::UpdateRewardingValidatorAddress { address } => {
            crate::mixnet_contract_settings::transactions::try_update_rewarding_validator_address(
                deps, info, address,
            )
        }
        ExecuteMsg::UpdateContractStateParams { updated_parameters } => {
            crate::mixnet_contract_settings::transactions::try_update_contract_settings(
                deps,
                info,
                updated_parameters,
            )
        }
        ExecuteMsg::UpdateActiveSetSize {
            active_set_size,
            force_immediately,
        } => crate::rewards::transactions::try_update_active_set_size(
            deps,
            env,
            info,
            active_set_size,
            force_immediately,
        ),
        ExecuteMsg::UpdateRewardingParams {
            updated_params,
            force_immediately,
        } => crate::rewards::transactions::try_update_rewarding_params(
            deps,
            env,
            info,
            updated_params,
            force_immediately,
        ),
        ExecuteMsg::UpdateIntervalConfig {
            epochs_in_interval,
            epoch_duration_secs,
            force_immediately,
        } => crate::interval::transactions::try_update_interval_config(
            deps,
            env,
            info,
            epochs_in_interval,
            epoch_duration_secs,
            force_immediately,
        ),
        ExecuteMsg::AdvanceCurrentEpoch {
            new_rewarded_set,
            expected_active_set_size,
        } => crate::interval::transactions::try_advance_epoch(
            deps,
            env,
            info,
            new_rewarded_set,
            expected_active_set_size,
        ),
        ExecuteMsg::ReconcileEpochEvents { limit } => {
            crate::interval::transactions::try_reconcile_epoch_events(deps, env, limit)
        }

        // mixnode-related:
        ExecuteMsg::BondMixnode {
            mix_node,
            cost_params,
            owner_signature,
        } => crate::mixnodes::transactions::try_add_mixnode(
            deps,
            env,
            info,
            mix_node,
            cost_params,
            owner_signature,
        ),
        ExecuteMsg::BondMixnodeOnBehalf {
            mix_node,
            cost_params,
            owner,
            owner_signature,
        } => crate::mixnodes::transactions::try_add_mixnode_on_behalf(
            deps,
            env,
            info,
            mix_node,
            cost_params,
            owner,
            owner_signature,
        ),
        ExecuteMsg::UnbondMixnode {} => {
            crate::mixnodes::transactions::try_remove_mixnode(deps, info)
        }
        ExecuteMsg::UnbondMixnodeOnBehalf { owner } => {
            crate::mixnodes::transactions::try_remove_mixnode_on_behalf(deps, info, owner)
        }
        ExecuteMsg::UpdateMixnodeCostParams { new_costs } => {
            crate::mixnodes::transactions::try_update_mixnode_cost_params(deps, info, new_costs)
        }
        ExecuteMsg::UpdateMixnodeCostParamsOnBehalf { new_costs, owner } => {
            crate::mixnodes::transactions::try_update_mixnode_cost_params_on_behalf(
                deps, info, new_costs, owner,
            )
        }
        ExecuteMsg::UpdateMixnodeConfig { new_config } => {
            crate::mixnodes::transactions::try_update_mixnode_config(deps, info, new_config)
        }
        ExecuteMsg::UpdateMixnodeConfigOnBehalf { new_config, owner } => {
            crate::mixnodes::transactions::try_update_mixnode_config_on_behalf(
                deps, info, new_config, owner,
            )
        }

        // gateway-related:
        ExecuteMsg::BondGateway {
            gateway,
            owner_signature,
        } => crate::gateways::transactions::try_add_gateway(
            deps,
            env,
            info,
            gateway,
            owner_signature,
        ),
        ExecuteMsg::BondGatewayOnBehalf {
            gateway,
            owner,
            owner_signature,
        } => crate::gateways::transactions::try_add_gateway_on_behalf(
            deps,
            env,
            info,
            gateway,
            owner,
            owner_signature,
        ),
        ExecuteMsg::UnbondGateway {} => {
            crate::gateways::transactions::try_remove_gateway(deps, info)
        }
        ExecuteMsg::UnbondGatewayOnBehalf { owner } => {
            crate::gateways::transactions::try_remove_gateway_on_behalf(deps, info, owner)
        }

        // delegation-related:
        ExecuteMsg::DelegateToMixnode { mix_id } => {
            crate::delegations::transactions::try_delegate_to_mixnode(deps, info, mix_id)
        }
        ExecuteMsg::DelegateToMixnodeOnBehalf { mix_id, delegate } => {
            crate::delegations::transactions::try_delegate_to_mixnode_on_behalf(
                deps, info, mix_id, delegate,
            )
        }
        ExecuteMsg::UndelegateFromMixnode { mix_id } => {
            crate::delegations::transactions::try_remove_delegation_from_mixnode(deps, info, mix_id)
        }
        ExecuteMsg::UndelegateFromMixnodeOnBehalf { mix_id, delegate } => {
            crate::delegations::transactions::try_remove_delegation_from_mixnode_on_behalf(
                deps, info, mix_id, delegate,
            )
        }

        // reward-related
        ExecuteMsg::RewardMixnode {
            mix_id,
            performance,
        } => crate::rewards::transactions::try_reward_mixnode(deps, env, info, mix_id, performance),

        ExecuteMsg::WithdrawOperatorReward {} => {
            crate::rewards::transactions::try_withdraw_operator_reward(deps, info)
        }
        ExecuteMsg::WithdrawOperatorRewardOnBehalf { owner } => {
            crate::rewards::transactions::try_withdraw_operator_reward_on_behalf(deps, info, owner)
        }
        ExecuteMsg::WithdrawDelegatorReward { mix_id } => {
            crate::rewards::transactions::try_withdraw_delegator_reward(deps, info, mix_id)
        }
        ExecuteMsg::WithdrawDelegatorRewardOnBehalf { mix_id, owner } => {
            crate::rewards::transactions::try_withdraw_delegator_reward_on_behalf(
                deps, info, mix_id, owner,
            )
        }
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

        // mixnode-related:
        QueryMsg::GetMixNodeBonds { start_after, limit } => to_binary(
            &crate::mixnodes::bonding_queries::query_mixnode_bonds_paged(deps, start_after, limit)?,
        ),
        QueryMsg::GetMixNodesDetailed { start_after, limit } => to_binary(
            &crate::mixnodes::bonding_queries::query_mixnodes_details_paged(
                deps,
                start_after,
                limit,
            )?,
        ),
        QueryMsg::GetUnbondedMixNodes { limit, start_after } => to_binary(
            &crate::mixnodes::bonding_queries::query_unbonded_mixnodes_paged(
                deps,
                start_after,
                limit,
            )?,
        ),
        QueryMsg::GetOwnedMixnode { address } => to_binary(
            &crate::mixnodes::bonding_queries::query_owned_mixnode(deps, address)?,
        ),
        QueryMsg::GetMixnodeDetails { mix_id } => to_binary(
            &crate::mixnodes::bonding_queries::query_mixnode_details(deps, mix_id)?,
        ),
        QueryMsg::GetUnbondedMixNodeInformation { mix_id } => to_binary(
            &crate::mixnodes::bonding_queries::query_unbonded_mixnode(deps, mix_id)?,
        ),
        QueryMsg::GetLayerDistribution {} => to_binary(
            &crate::mixnodes::layer_queries::query_layer_distribution(deps)?,
        ),

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
            proxy,
            start_after,
            limit,
        } => to_binary(
            &crate::delegations::queries::query_delegator_delegations_paged(
                deps,
                delegator,
                proxy,
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

        // QueryMsg::GetRewardPool {} => to_binary(&query_reward_pool(deps)?),
        // QueryMsg::GetCirculatingSupply {} => to_binary(&query_circulating_supply(deps)?),
        // QueryMsg::GetStakingSupply {} => to_binary(&query_staking_supply(deps)?),
        // QueryMsg::GetIntervalRewardPercent {} => to_binary(&INTERVAL_REWARD_PERCENT),
        // QueryMsg::GetSybilResistancePercent {} => to_binary(&SYBIL_RESISTANCE_PERCENT),
        // QueryMsg::GetActiveSetWorkFactor {} => to_binary(&ACTIVE_SET_WORK_FACTOR),
        // QueryMsg::GetRewardingStatus {
        //     mix_identity,
        //     interval_id,
        // } => to_binary(&query_rewarding_status(deps, mix_identity, interval_id)?),
        // QueryMsg::GetRewardedSet {
        //     height,
        //     start_after,
        //     limit,
        // } => to_binary(&query_rewarded_set(
        //     deps.storage,
        //     height,
        //     start_after,
        //     limit,
        // )?),
        // QueryMsg::GetRewardedSetUpdateDetails {} => {
        //     to_binary(&query_rewarded_set_update_details(env, deps.storage)?)
        // }
        // QueryMsg::GetCurrentRewardedSetHeight {} => {
        //     to_binary(&query_current_rewarded_set_height(deps.storage)?)
        // }
        // // QueryMsg::GetCurrentInterval {} => to_binary(&query_current_interval(deps.storage)?),
        // QueryMsg::GetRewardedSetRefreshBlocks {} => {
        //     to_binary(&query_rewarded_set_refresh_minimum_blocks())
        // }
        // QueryMsg::GetEpochsInInterval {} => {
        //     to_binary(&crate::support::helpers::epochs_in_interval(deps.storage)?)
        // }
        // QueryMsg::GetCurrentOperatorCost {} => to_binary(
        //     &crate::support::helpers::current_operator_epoch_cost(deps.storage)?,
        // ),
        // QueryMsg::GetCurrentEpoch {} => to_binary(&query_current_epoch(deps.storage)?),
        // QueryMsg::QueryOperatorReward { address } => to_binary(
        //     &crate::rewards::queries::query_operator_reward(deps, address)?,
        // ),
        // QueryMsg::QueryDelegatorReward {
        //     address,
        //     mix_identity,
        //     proxy,
        // } => to_binary(&crate::rewards::queries::query_delegator_reward(
        //     deps,
        //     address,
        //     mix_identity,
        //     proxy,
        // )?),
        // QueryMsg::GetPendingDelegationEvents {
        //     owner_address,
        //     proxy_address,
        // } => to_binary(&query_pending_delegation_events(
        //     deps,
        //     owner_address,
        //     proxy_address,
        // )?),
        // QueryMsg::GetAllDelegationKeys {} => to_binary(
        //     &crate::delegations::queries::query_all_delegation_keys(deps.storage)?,
        // ),
        // QueryMsg::DebugGetAllDelegationValues {} => to_binary(
        //     &crate::delegations::queries::debug_query_all_delegation_values(deps.storage)?,
        // ),
        // QueryMsg::GetCheckpointsForMixnode { mix_identity } => {
        //     to_binary(&query_checkpoints_for_mixnode(deps, mix_identity)?)
        // }
        // QueryMsg::GetMixnodeAtHeight {
        //     mix_identity,
        //     height,
        // } => to_binary(&query_mixnode_at_height(deps, mix_identity, height)?),
        _ => todo!(),
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

// #[cfg(test)]
// pub mod tests {
//     use super::*;
//     use crate::support::tests;
//     use config::defaults::{DEFAULT_NETWORK, MIX_DENOM};
//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
//     use cosmwasm_std::{coins, from_binary};
//     use mixnet_contract_common::PagedMixnodeResponse;
//
//     #[test]
//     fn initialize_contract() {
//         let mut deps = mock_dependencies();
//         let env = mock_env();
//         let msg = InstantiateMsg {
//             rewarding_validator_address: DEFAULT_NETWORK.rewarding_validator_address().to_string(),
//         };
//         let info = mock_info("creator", &[]);
//
//         let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
//         assert_eq!(0, res.messages.len());
//
//         // mix_node_bonds should be empty after initialization
//         let res = query(
//             deps.as_ref(),
//             env.clone(),
//             QueryMsg::GetMixNodes {
//                 start_after: None,
//                 limit: Option::from(2),
//             },
//         )
//         .unwrap();
//         let page: PagedMixnodeResponse = from_binary(&res).unwrap();
//         assert_eq!(0, page.nodes.len()); // there are no mixnodes in the list when it's just been initialized
//
//         // Contract balance should match what we initialized it as
//         assert_eq!(
//             coins(0, MIX_DENOM.base),
//             tests::queries::query_contract_balance(env.contract.address, deps)
//         );
//     }
// }
