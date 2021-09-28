// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::calculate_epoch_reward_rate;
use crate::state::State;
use crate::storage::{config, layer_distribution};
use crate::{error::ContractError, queries, transactions};
use config::defaults::NETWORK_MONITOR_ADDRESS;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Decimal, Deps, DepsMut, Env, MessageInfo, QueryResponse,
    Response, Uint128,
};
use cosmwasm_storage::singleton_read;
use mixnet_contract::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StateParams};
use serde::{Deserialize, Serialize};

pub const INITIAL_DEFAULT_EPOCH_LENGTH: u32 = 2;

/// Constant specifying minimum of coin required to bond a gateway
pub const INITIAL_GATEWAY_BOND: Uint128 = Uint128(100_000000);

/// Constant specifying minimum of coin required to bond a mixnode
pub const INITIAL_MIXNODE_BOND: Uint128 = Uint128(100_000000);

// percentage annual increase. Given starting value of x, we expect to have 1.1x at the end of the year
pub const INITIAL_MIXNODE_BOND_REWARD_RATE: u64 = 110;
pub const INITIAL_GATEWAY_BOND_REWARD_RATE: u64 = 110;
pub const INITIAL_MIXNODE_DELEGATION_REWARD_RATE: u64 = 110;
pub const INITIAL_GATEWAY_DELEGATION_REWARD_RATE: u64 = 110;

pub const INITIAL_MIXNODE_ACTIVE_SET_SIZE: u32 = 100;
pub const INITIAL_GATEWAY_ACTIVE_SET_SIZE: u32 = 20;

fn default_initial_state(owner: Addr) -> State {
    let mixnode_bond_reward_rate = Decimal::percent(INITIAL_MIXNODE_BOND_REWARD_RATE);
    let gateway_bond_reward_rate = Decimal::percent(INITIAL_GATEWAY_BOND_REWARD_RATE);
    let mixnode_delegation_reward_rate = Decimal::percent(INITIAL_MIXNODE_DELEGATION_REWARD_RATE);
    let gateway_delegation_reward_rate = Decimal::percent(INITIAL_GATEWAY_DELEGATION_REWARD_RATE);

    State {
        owner,
        network_monitor_address: Addr::unchecked(NETWORK_MONITOR_ADDRESS), // we trust our hardcoded value
        params: StateParams {
            epoch_length: INITIAL_DEFAULT_EPOCH_LENGTH,
            minimum_mixnode_bond: INITIAL_MIXNODE_BOND,
            minimum_gateway_bond: INITIAL_GATEWAY_BOND,
            mixnode_bond_reward_rate,
            gateway_bond_reward_rate,
            mixnode_delegation_reward_rate,
            gateway_delegation_reward_rate,
            mixnode_active_set_size: INITIAL_MIXNODE_ACTIVE_SET_SIZE,
            gateway_active_set_size: INITIAL_GATEWAY_ACTIVE_SET_SIZE,
        },
        mixnode_epoch_bond_reward: calculate_epoch_reward_rate(
            INITIAL_DEFAULT_EPOCH_LENGTH,
            mixnode_bond_reward_rate,
        ),
        gateway_epoch_bond_reward: calculate_epoch_reward_rate(
            INITIAL_DEFAULT_EPOCH_LENGTH,
            gateway_bond_reward_rate,
        ),
        mixnode_epoch_delegation_reward: calculate_epoch_reward_rate(
            INITIAL_DEFAULT_EPOCH_LENGTH,
            mixnode_delegation_reward_rate,
        ),
        gateway_epoch_delegation_reward: calculate_epoch_reward_rate(
            INITIAL_DEFAULT_EPOCH_LENGTH,
            gateway_delegation_reward_rate,
        ),
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
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = default_initial_state(info.sender);

    config(deps.storage).save(&state)?;
    layer_distribution(deps.storage).save(&Default::default())?;
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
            transactions::try_add_mixnode(deps, env, info, mix_node)
        }
        ExecuteMsg::UnbondMixnode {} => transactions::try_remove_mixnode(deps, info),
        ExecuteMsg::BondGateway { gateway } => {
            transactions::try_add_gateway(deps, env, info, gateway)
        }
        ExecuteMsg::UnbondGateway {} => transactions::try_remove_gateway(deps, info),
        ExecuteMsg::UpdateStateParams(params) => {
            transactions::try_update_state_params(deps, info, params)
        }
        ExecuteMsg::RewardMixnode { identity, uptime } => {
            transactions::try_reward_mixnode(deps, env, info, identity, uptime)
        }
        ExecuteMsg::RewardGateway { identity, uptime } => {
            transactions::try_reward_gateway(deps, env, info, identity, uptime)
        }
        ExecuteMsg::DelegateToMixnode { mix_identity } => {
            transactions::try_delegate_to_mixnode(deps, env, info, mix_identity)
        }
        ExecuteMsg::UndelegateFromMixnode { mix_identity } => {
            transactions::try_remove_delegation_from_mixnode(deps, info, mix_identity)
        }
        ExecuteMsg::DelegateToGateway { gateway_identity } => {
            transactions::try_delegate_to_gateway(deps, env, info, gateway_identity)
        }
        ExecuteMsg::UndelegateFromGateway { gateway_identity } => {
            transactions::try_remove_delegation_from_gateway(deps, info, gateway_identity)
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let query_res = match msg {
        QueryMsg::GetMixNodes { start_after, limit } => {
            to_binary(&queries::query_mixnodes_paged(deps, start_after, limit)?)
        }
        QueryMsg::GetGateways { limit, start_after } => {
            to_binary(&queries::query_gateways_paged(deps, start_after, limit)?)
        }
        QueryMsg::OwnsMixnode { address } => {
            to_binary(&queries::query_owns_mixnode(deps, address)?)
        }
        QueryMsg::OwnsGateway { address } => {
            to_binary(&queries::query_owns_gateway(deps, address)?)
        }
        QueryMsg::StateParams {} => to_binary(&queries::query_state_params(deps)),
        QueryMsg::LayerDistribution {} => to_binary(&queries::query_layer_distribution(deps)),
        QueryMsg::GetMixDelegations {
            mix_identity,
            start_after,
            limit,
        } => to_binary(&queries::query_mixnode_delegations_paged(
            deps,
            mix_identity,
            start_after,
            limit,
        )?),
        QueryMsg::GetReverseMixDelegations {
            delegation_owner,
            start_after,
            limit,
        } => to_binary(&queries::query_reverse_mixnode_delegations_paged(
            deps,
            delegation_owner,
            start_after,
            limit,
        )?),
        QueryMsg::GetMixDelegation {
            mix_identity,
            address,
        } => to_binary(&queries::query_mixnode_delegation(
            deps,
            mix_identity,
            address,
        )?),
        QueryMsg::GetGatewayDelegations {
            gateway_identity,
            start_after,
            limit,
        } => to_binary(&queries::query_gateway_delegations_paged(
            deps,
            gateway_identity,
            start_after,
            limit,
        )?),
        QueryMsg::GetReverseGatewayDelegations {
            delegation_owner,
            start_after,
            limit,
        } => to_binary(&queries::query_reverse_gateway_delegations_paged(
            deps,
            delegation_owner,
            start_after,
            limit,
        )?),
        QueryMsg::GetGatewayDelegation {
            gateway_identity,
            address,
        } => to_binary(&queries::query_gateway_delegation(
            deps,
            gateway_identity,
            address,
        )?),
    };

    Ok(query_res?)
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    #[derive(Serialize, Deserialize)]
    struct OldStateParams {
        epoch_length: u32,
        minimum_mixnode_bond: Uint128,
        minimum_gateway_bond: Uint128,
        mixnode_bond_reward_rate: Decimal,
        gateway_bond_reward_rate: Decimal,
        mixnode_delegation_reward_rate: Decimal,
        gateway_delegation_reward_rate: Decimal,
        mixnode_active_set_size: u32,
    }

    // adding gateways active set
    #[derive(Serialize, Deserialize)]
    struct OldState {
        owner: Addr,
        network_monitor_address: Addr,
        params: OldStateParams,
        mixnode_epoch_bond_reward: Decimal,
        gateway_epoch_bond_reward: Decimal,
        mixnode_epoch_delegation_reward: Decimal,
        gateway_epoch_delegation_reward: Decimal,
    }

    let old_state: OldState = singleton_read(deps.storage, b"config").load()?;

    let new_state = State {
        owner: old_state.owner,
        network_monitor_address: old_state.network_monitor_address,
        params: StateParams {
            epoch_length: old_state.params.epoch_length,
            minimum_mixnode_bond: old_state.params.minimum_mixnode_bond,
            minimum_gateway_bond: old_state.params.minimum_gateway_bond,
            mixnode_bond_reward_rate: old_state.params.mixnode_bond_reward_rate,
            gateway_bond_reward_rate: old_state.params.gateway_bond_reward_rate,
            mixnode_delegation_reward_rate: old_state.params.mixnode_delegation_reward_rate,
            gateway_delegation_reward_rate: old_state.params.gateway_delegation_reward_rate,
            mixnode_active_set_size: old_state.params.mixnode_active_set_size,
            gateway_active_set_size: INITIAL_GATEWAY_ACTIVE_SET_SIZE,
        },
        mixnode_epoch_bond_reward: old_state.mixnode_epoch_bond_reward,
        gateway_epoch_bond_reward: old_state.gateway_epoch_bond_reward,
        mixnode_epoch_delegation_reward: old_state.mixnode_epoch_delegation_reward,
        gateway_epoch_delegation_reward: old_state.gateway_epoch_delegation_reward,
    };

    config(deps.storage).save(&new_state)?;

    Ok(Default::default())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::support::tests::helpers::*;
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};
    use mixnet_contract::PagedMixnodeResponse;

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies(&[]);
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
            query_contract_balance(env.contract.address, deps)
        );
    }
}
