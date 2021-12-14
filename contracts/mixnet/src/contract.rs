// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegations::queries::query_all_network_delegations_paged;
use crate::delegations::queries::query_delegator_delegations_paged;
use crate::delegations::queries::query_mixnode_delegation;
use crate::delegations::queries::query_mixnode_delegations_paged;
use crate::error::ContractError;
use crate::gateways::queries::query_gateways_paged;
use crate::gateways::queries::query_owns_gateway;
use crate::mixnet_contract_settings::models::ContractState;
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
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response, Uint128,
};
use mixnet_contract::{ContractStateParams, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

/// Constant specifying minimum of coin required to bond a gateway
pub const INITIAL_GATEWAY_PLEDGE: Uint128 = Uint128::new(100_000_000);

/// Constant specifying minimum of coin required to bond a mixnode
pub const INITIAL_MIXNODE_PLEDGE: Uint128 = Uint128::new(100_000_000);

pub const INITIAL_MIXNODE_REWARDED_SET_SIZE: u32 = 200;
pub const INITIAL_MIXNODE_ACTIVE_SET_SIZE: u32 = 100;

pub const INITIAL_REWARD_POOL: u128 = 250_000_000_000_000;
pub const EPOCH_REWARD_PERCENT: u8 = 2; // Used to calculate epoch reward pool
pub const DEFAULT_SYBIL_RESISTANCE_PERCENT: u8 = 30;
pub const DEFAULT_ACTIVE_SET_WORK_FACTOR: u8 = 10;

fn default_initial_state(
    owner: Addr,
    rewarding_validator_address: Addr,
    env: Env,
) -> ContractState {
    ContractState {
        owner,
        rewarding_validator_address,
        params: ContractStateParams {
            minimum_mixnode_pledge: INITIAL_MIXNODE_PLEDGE,
            minimum_gateway_pledge: INITIAL_GATEWAY_PLEDGE,
            mixnode_rewarded_set_size: INITIAL_MIXNODE_REWARDED_SET_SIZE,
            mixnode_active_set_size: INITIAL_MIXNODE_ACTIVE_SET_SIZE,
            active_set_work_factor: DEFAULT_ACTIVE_SET_WORK_FACTOR,
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
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let rewarding_validator_address = deps.api.addr_validate(&msg.rewarding_validator_address)?;
    let state = default_initial_state(info.sender, rewarding_validator_address, env);

    mixnet_params_storage::CONTRACT_STATE.save(deps.storage, &state)?;
    mixnet_params_storage::LAYERS.save(deps.storage, &Default::default())?;
    rewards_storage::REWARD_POOL.save(deps.storage, &Uint128::new(INITIAL_REWARD_POOL))?;

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
        ExecuteMsg::BondMixnode {
            mix_node,
            owner_signature,
        } => crate::mixnodes::transactions::try_add_mixnode(
            deps,
            env,
            info,
            mix_node,
            owner_signature,
        ),
        ExecuteMsg::UnbondMixnode {} => {
            crate::mixnodes::transactions::try_remove_mixnode(deps, info)
        }
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
        ExecuteMsg::UnbondGateway {} => {
            crate::gateways::transactions::try_remove_gateway(deps, info)
        }
        ExecuteMsg::UpdateContractStateParams(params) => {
            crate::mixnet_contract_settings::transactions::try_update_contract_settings(
                deps, info, params,
            )
        }
        ExecuteMsg::RewardMixnode {
            identity,
            params,
            rewarding_interval_nonce,
        } => crate::rewards::transactions::try_reward_mixnode(
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
        } => crate::rewards::transactions::try_reward_next_mixnode_delegators(
            deps,
            info,
            mix_identity,
            rewarding_interval_nonce,
        ),
        ExecuteMsg::DelegateToMixnodeOnBehalf {
            mix_identity,
            delegate,
        } => crate::delegations::transactions::try_delegate_to_mixnode_on_behalf(
            deps,
            env,
            info,
            mix_identity,
            delegate,
        ),
        ExecuteMsg::UndelegateFromMixnodeOnBehalf {
            mix_identity,
            delegate,
        } => crate::delegations::transactions::try_remove_delegation_from_mixnode_on_behalf(
            deps,
            info,
            mix_identity,
            delegate,
        ),
        ExecuteMsg::BondMixnodeOnBehalf {
            mix_node,
            owner,
            owner_signature,
        } => crate::mixnodes::transactions::try_add_mixnode_on_behalf(
            deps,
            env,
            info,
            mix_node,
            owner,
            owner_signature,
        ),
        ExecuteMsg::UnbondMixnodeOnBehalf { owner } => {
            crate::mixnodes::transactions::try_remove_mixnode_on_behalf(deps, info, owner)
        }
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
        ExecuteMsg::UnbondGatewayOnBehalf { owner } => {
            crate::gateways::transactions::try_remove_gateway_on_behalf(deps, info, owner)
        }
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
        QueryMsg::GetRewardPool {} => to_binary(&query_reward_pool(deps)?),
        QueryMsg::GetCirculatingSupply {} => to_binary(&query_circulating_supply(deps)?),
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
        let msg = InstantiateMsg {
            rewarding_validator_address: config::defaults::REWARDING_VALIDATOR_ADDRESS.to_string(),
        };
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
