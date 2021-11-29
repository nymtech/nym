// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegations::queries::query_all_network_delegations_paged;
use crate::delegations::queries::query_delegator_delegations_paged;
use crate::delegations::queries::query_mixnode_delegation;
use crate::delegations::queries::query_mixnode_delegations_paged;
use crate::error::ContractError;
use crate::gateways::queries::query_gateways_paged;
use crate::gateways::queries::query_owns_gateway;
use crate::mixnet_contract_settings::models::ContractSettings;
use crate::mixnet_contract_settings::queries::query_rewarding_interval;
use crate::mixnet_contract_settings::queries::{
    query_contract_settings_params, query_contract_version,
};
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::bonding_queries as mixnode_queries;
use crate::mixnodes::bonding_queries::query_mixnodes_paged;
use crate::mixnodes::layer_queries::query_layer_distribution;
use crate::rewards::queries::query_reward_pool;
use crate::rewards::queries::{query_circulating_supply, query_rewarding_status};
use config::defaults::REWARDING_VALIDATOR_ADDRESS;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response, Uint128,
};
use mixnet_contract::{ContractSettingsParams, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use std::u128;

/// Constant specifying minimum of coin required to bond a gateway
pub const INITIAL_GATEWAY_BOND: Uint128 = Uint128::new(100_000_000);

/// Constant specifying minimum of coin required to bond a mixnode
pub const INITIAL_MIXNODE_BOND: Uint128 = Uint128::new(100_000_000);

pub const INITIAL_MIXNODE_REWARDED_SET_SIZE: u32 = 200;
pub const INITIAL_MIXNODE_ACTIVE_SET_SIZE: u32 = 100;

pub const INITIAL_REWARD_POOL: u128 = 250_000_000_000_000;
pub const EPOCH_REWARD_PERCENT: u8 = 2; // Used to calculate epoch reward pool
pub const DEFAULT_SYBIL_RESISTANCE_PERCENT: u8 = 30;

// We'll be assuming a few more things, profit margin and cost function. Since we don't have reliable package measurement, we'll be using uptime. We'll also set the value of 1 Nym to 1 $, to be able to translate epoch costs to Nyms. We'll also assume a cost of 40$ per epoch(month), converting that to Nym at our 1$ rate translates to 40_000_000 uNyms
pub const DEFAULT_COST_PER_EPOCH: u32 = 40_000_000;

fn default_initial_state(owner: Addr, env: Env) -> ContractSettings {
    ContractSettings {
        owner,
        rewarding_validator_address: Addr::unchecked(REWARDING_VALIDATOR_ADDRESS), // we trust our hardcoded value
        params: ContractSettingsParams {
            minimum_mixnode_bond: INITIAL_MIXNODE_BOND,
            minimum_gateway_bond: INITIAL_GATEWAY_BOND,
            mixnode_rewarded_set_size: INITIAL_MIXNODE_REWARDED_SET_SIZE,
            mixnode_active_set_size: INITIAL_MIXNODE_ACTIVE_SET_SIZE,
        },
        rewarding_interval_starting_block: env.block.height,
        latest_rewarding_interval_nonce: 0,
        rewarding_in_progress: false,
    }
}

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = default_initial_state(info.sender, env);

    mixnet_params_storage::CONTRACT_SETTINGS.save(deps.storage, &state)?;
    mixnet_params_storage::LAYERS.save(deps.storage, &Default::default())?;
    Ok(Response::default())
}

/// Handle an incoming message
#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::BondMixnode { mix_node } => {
            crate::mixnodes::transactions::try_add_mixnode(deps, env, info, mix_node)
        }
        ExecuteMsg::UnbondMixnode {} => {
            crate::mixnodes::transactions::try_remove_mixnode(deps, info)
        }
        ExecuteMsg::BondGateway { gateway } => {
            crate::gateways::transactions::try_add_gateway(deps, env, info, gateway)
        }
        ExecuteMsg::UnbondGateway {} => {
            crate::gateways::transactions::try_remove_gateway(deps, info)
        }
        ExecuteMsg::UpdateContractSettings(params) => {
            crate::mixnet_contract_settings::transactions::try_update_contract_settings(
                deps, info, params,
            )
        }
        ExecuteMsg::RewardMixnodeV2 {
            identity,
            params,
            rewarding_interval_nonce,
        } => crate::rewards::transactions::try_reward_mixnode_v2(
            deps,
            env,
            info,
            identity,
            params,
            rewarding_interval_nonce,
        ),
        ExecuteMsg::DelegateToMixnode { mix_identity } => {
            crate::delegations::transactions::try_delegate_to_mixnode(deps, env, info, mix_identity)
        }
        ExecuteMsg::UndelegateFromMixnode { mix_identity } => {
            crate::delegations::transactions::try_remove_delegation_from_mixnode(
                deps,
                info,
                mix_identity,
            )
        }
        ExecuteMsg::BeginMixnodeRewarding {
            rewarding_interval_nonce,
        } => crate::rewards::transactions::try_begin_mixnode_rewarding(
            deps,
            env,
            info,
            rewarding_interval_nonce,
        ),
        ExecuteMsg::FinishMixnodeRewarding {
            rewarding_interval_nonce,
        } => crate::rewards::transactions::try_finish_mixnode_rewarding(
            deps,
            info,
            rewarding_interval_nonce,
        ),
        ExecuteMsg::RewardNextMixDelegators {
            mix_identity,
            rewarding_interval_nonce,
        } => crate::rewards::transactions::try_reward_next_mixnode_delegators_v2(
            deps,
            info,
            mix_identity,
            rewarding_interval_nonce,
        ),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let query_res = match msg {
        QueryMsg::GetContractVersion {} => to_binary(&query_contract_version()),
        QueryMsg::GetMixNodes { start_after, limit } => {
            to_binary(&query_mixnodes_paged(deps, start_after, limit)?)
        }
        QueryMsg::GetGateways { limit, start_after } => {
            to_binary(&query_gateways_paged(deps, start_after, limit)?)
        }
        QueryMsg::OwnsMixnode { address } => {
            to_binary(&mixnode_queries::query_owns_mixnode(deps, address)?)
        }
        QueryMsg::OwnsGateway { address } => to_binary(&query_owns_gateway(deps, address)?),
        QueryMsg::StateParams {} => to_binary(&query_contract_settings_params(deps)?),
        QueryMsg::CurrentRewardingInterval {} => to_binary(&query_rewarding_interval(deps)?),
        QueryMsg::LayerDistribution {} => to_binary(&query_layer_distribution(deps)?),
        QueryMsg::GetMixnodeDelegations {
            mix_identity,
            start_after,
            limit,
        } => to_binary(&query_mixnode_delegations_paged(
            deps,
            mix_identity,
            start_after,
            limit,
        )?),
        QueryMsg::GetAllNetworkDelegations { start_after, limit } => to_binary(
            &query_all_network_delegations_paged(deps, start_after, limit)?,
        ),
        QueryMsg::GetDelegatorDelegations {
            delegator: delegation_owner,
            start_after,
            limit,
        } => to_binary(&query_delegator_delegations_paged(
            deps,
            delegation_owner,
            start_after,
            limit,
        )?),
        QueryMsg::GetDelegationDetails {
            mix_identity,
            delegator,
        } => to_binary(&query_mixnode_delegation(deps, mix_identity, delegator)?),
        QueryMsg::GetRewardPool {} => to_binary(&query_reward_pool(deps)),
        QueryMsg::GetCirculatingSupply {} => to_binary(&query_circulating_supply(deps)),
        QueryMsg::GetEpochRewardPercent {} => to_binary(&EPOCH_REWARD_PERCENT),
        QueryMsg::GetSybilResistancePercent {} => to_binary(&DEFAULT_SYBIL_RESISTANCE_PERCENT),
        QueryMsg::GetRewardingStatus {
            mix_identity,
            rewarding_interval_nonce,
        } => to_binary(&query_rewarding_status(
            deps,
            mix_identity,
            rewarding_interval_nonce,
        )?),
    };

    Ok(query_res?)
}
#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Default::default())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::support::tests::test_helpers;
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};
    use mixnet_contract::PagedMixnodeResponse;

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // mix_node_bonds should be empty after initialization
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedMixnodeResponse = from_binary(&res).unwrap();
        assert_eq!(0, page.nodes.len()); // there are no mixnodes in the list when it's just been initialized

        // Contract balance should match what we initialized it as
        assert_eq!(
            coins(0, DENOM),
            test_helpers::query_contract_balance(env.contract.address, deps)
        );
    }
}
