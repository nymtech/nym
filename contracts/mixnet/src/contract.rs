// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::u128;

use crate::helpers::calculate_epoch_reward_rate;
use crate::state::State;
use crate::storage::{config, layer_distribution};
use crate::{error::ContractError, queries, transactions};
use config::defaults::NETWORK_MONITOR_ADDRESS;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Decimal, Deps, DepsMut, Env, MessageInfo, QueryResponse,
    Response, Uint128,
};
use mixnet_contract::{ExecuteMsg, InstantiateMsg, MigrateMsg, MixNode, QueryMsg, StateParams};

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

// This is totally made up, lets set the pool to billion nyms, so million billion micro nyms
pub const INITIAL_INFLATION_POOL: u128 = 1_000_000_000_000_000;
// We'll be assuming a few more things, profit margin and cost function. Since we don't have relialable package measurement, we'll be using uptime. We'll also set the value of 1 Nym to 1 $, to be able to translate epoch costs to Nyms. We'll also assume a cost of 40$ per epoch(month), converting that to Nym at our 1$ rate translates to 40_000_000 uNyms
pub const DEFAULT_COST_PER_EPOCH: u32 = 40_000_000;

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
        ExecuteMsg::RewardMixnodeV2 { identity, params } => {
            transactions::try_reward_mixnode_v2(deps, env, info, identity, params)
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
        QueryMsg::GetAllMixDelegations { start_after, limit } => to_binary(
            &queries::query_all_mixnode_delegations_paged(deps, start_after, limit)?,
        ),
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
        QueryMsg::GetAllGatewayDelegations { start_after, limit } => to_binary(
            &queries::query_all_gateway_delegations_paged(deps, start_after, limit)?,
        ),
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
        QueryMsg::GetTotalMixStake {} => to_binary(&queries::query_total_mix_stake(deps)),
        QueryMsg::GetTotalGatewayStake {} => to_binary(&queries::query_total_gt_stake(deps)),
        QueryMsg::GetInflationPool {} => to_binary(&queries::query_inflation_pool(deps)),
    };

    Ok(query_res?)
}
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    use crate::storage::{
        gateways_read, incr_total_gateway_stake, incr_total_mix_stake, mixnodes, PREFIX_MIXNODES,
    };
    use cosmwasm_std::{Coin, Order, StdResult};
    use cosmwasm_storage::bucket_read;
    use mixnet_contract::{GatewayBond, Layer, MixNodeBond};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct OldMixNodeBond {
        pub bond_amount: Coin,
        pub total_delegation: Coin,
        pub owner: Addr,
        pub layer: Layer,
        pub block_height: u64,
        pub mix_node: MixNode,
    }

    impl From<OldMixNodeBond> for MixNodeBond {
        fn from(o: OldMixNodeBond) -> MixNodeBond {
            MixNodeBond {
                bond_amount: o.bond_amount,
                total_delegation: o.total_delegation,
                owner: o.owner,
                layer: o.layer,
                block_height: o.block_height,
                mix_node: o.mix_node,
                profit_margin_percent: 10,
            }
        }
    }

    let mixnode_bonds = bucket_read(deps.storage, PREFIX_MIXNODES)
        .range(None, None, Order::Ascending)
        .take_while(Result::is_ok)
        .map(Result::unwrap)
        .map(|(key, bond): (Vec<u8>, OldMixNodeBond)| (key, bond.into()))
        .collect::<Vec<(Vec<u8>, MixNodeBond)>>();

    for (key, bond) in mixnode_bonds {
        incr_total_mix_stake(bond.bond_amount().amount, deps.storage)?;
        incr_total_mix_stake(bond.total_delegation().amount, deps.storage)?;
        mixnodes(deps.storage).save(&key, &bond)?;
    }

    let gateway_bonds = gateways_read(deps.storage)
        .range(None, None, Order::Ascending)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<GatewayBond>>>()?;

    for bond in gateway_bonds {
        incr_total_gateway_stake(bond.bond_amount().amount, deps.storage)?;
        incr_total_gateway_stake(bond.total_delegation().amount, deps.storage)?;
    }

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
