use crate::helpers::calculate_epoch_reward_rate;
use crate::msg::{HandleMsg, InitMsg, MigrateMsg, QueryMsg};
use crate::state::{State, StateParams};
use crate::storage::{
    config, gateways, gateways_owners, layer_distribution, mixnodes, mixnodes_owners,
};
use crate::{error::ContractError, queries, transactions};
use cosmwasm_std::{
    to_binary, Decimal, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo,
    MigrateResponse, QueryResponse, StdResult, Uint128,
};
use mixnet_contract::{GatewayBond, MixNodeBond};

pub const INITIAL_DEFAULT_EPOCH_LENGTH: u32 = 2;

/// Constant specifying minimum of coin required to bond a gateway
pub const INITIAL_GATEWAY_BOND: Uint128 = Uint128(100_000000);

/// Constant specifying minimum of coin required to bond a mixnode
pub const INITIAL_MIXNODE_BOND: Uint128 = Uint128(100_000000);

// percentage annual increase. Given starting value of x, we expect to have 1.1x at the end of the year
pub const INITIAL_MIXNODE_BOND_REWARD_RATE: u64 = 110;
pub const INITIAL_GATEWAY_BOND_REWARD_RATE: u64 = 110;

pub const INITIAL_MIXNODE_ACTIVE_SET_SIZE: u32 = 100;

const NETWORK_MONITOR_ADDRESS: &str = "hal1v9qauwdq5terag6uvfsdytcs2d0sdmfdq6e83g";

/// Constant specifying denomination of the coin used for bonding
pub const DENOM: &str = "uhal";

fn default_initial_state(owner: HumanAddr) -> State {
    let mixnode_bond_reward_rate = Decimal::percent(INITIAL_MIXNODE_BOND_REWARD_RATE);
    let gateway_bond_reward_rate = Decimal::percent(INITIAL_GATEWAY_BOND_REWARD_RATE);

    State {
        owner,
        network_monitor_address: NETWORK_MONITOR_ADDRESS.into(),
        params: StateParams {
            epoch_length: INITIAL_DEFAULT_EPOCH_LENGTH,
            minimum_mixnode_bond: INITIAL_MIXNODE_BOND,
            minimum_gateway_bond: INITIAL_GATEWAY_BOND,
            mixnode_bond_reward_rate,
            gateway_bond_reward_rate,
            mixnode_active_set_size: INITIAL_MIXNODE_ACTIVE_SET_SIZE,
        },
        mixnode_epoch_bond_reward: calculate_epoch_reward_rate(
            INITIAL_DEFAULT_EPOCH_LENGTH,
            mixnode_bond_reward_rate,
        ),
        gateway_epoch_bond_reward: calculate_epoch_reward_rate(
            INITIAL_DEFAULT_EPOCH_LENGTH,
            gateway_bond_reward_rate,
        ),
    }
}

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    let state = default_initial_state(info.sender);

    config(deps.storage).save(&state)?;
    layer_distribution(deps.storage).save(&Default::default())?;
    Ok(InitResponse::default())
}

/// Handle an incoming message
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::BondMixnode { mix_node } => transactions::try_add_mixnode(deps, info, mix_node),
        HandleMsg::UnbondMixnode { mix_identity } => {
            transactions::try_remove_mixnode(deps, info, env, mix_identity)
        }
        HandleMsg::BondGateway { gateway } => transactions::try_add_gateway(deps, info, gateway),
        HandleMsg::UnbondGateway { gateway_identity } => {
            transactions::try_remove_gateway(deps, info, env, gateway_identity)
        }
        HandleMsg::UpdateStateParams(params) => {
            transactions::try_update_state_params(deps, info, params)
        }
        HandleMsg::RewardMixnode { identity, uptime } => {
            transactions::try_reward_mixnode(deps, info, identity, uptime)
        }
        HandleMsg::RewardGateway { identity, uptime } => {
            transactions::try_reward_gateway(deps, info, identity, uptime)
        }
        HandleMsg::DelegateToMixnode { mix_identity } => {
            transactions::try_delegate_to_mixnode(deps, info, mix_identity)
        }
        HandleMsg::UndelegateFromMixnode { mix_identity } => {
            transactions::try_remove_delegation_from_mixnode(deps, info, env, mix_identity)
        }
        HandleMsg::DelegateToGateway { gateway_identity } => {
            transactions::try_delegate_to_gateway(deps, info, gateway_identity)
        }
        HandleMsg::UndelegateFromGateway { gateway_identity } => {
            transactions::try_remove_delegation_from_gateway(deps, info, env, gateway_identity)
        }
    }
}

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

pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: MigrateMsg,
) -> Result<MigrateResponse, ContractError> {
    // Mixnode migration

    // What we do here for mixnodes is the following (the procedure is be identical for gateways):
    // 1. Load mixnodes (page by page) using the PREFIX_MIXNODES_OLD
    // 2. Save that the same data using PREFIX_MIXNODES, but the data key will be the value.mix_node.identity instead
    // 3. Load mixnode owners (page by page) using PREFIX_MIXNODES_OWNERS_OLD
    // 4. Save the data in reverse order using PREFIX_MIXNODES_OWNERS such that the key becomes the value and vice versa

    use cosmwasm_std::Order;
    use cosmwasm_std::Storage;
    use cosmwasm_storage::{bucket, Bucket};

    // those shouldn't be accessible ANYWHERE outside the migration
    const PREFIX_MIXNODES_OLD: &[u8] = b"mixnodes";
    const PREFIX_MIXNODES_OWNERS_OLD: &[u8] = b"mix-owners";
    const PREFIX_GATEWAYS_OLD: &[u8] = b"gateways";
    const PREFIX_GATEWAYS_OWNERS_OLD: &[u8] = b"gateway-owners";

    fn mixnodes_owners_old(storage: &mut dyn Storage) -> Bucket<HumanAddr> {
        bucket(storage, PREFIX_MIXNODES_OWNERS_OLD)
    }

    fn gateways_owners_old(storage: &mut dyn Storage) -> Bucket<HumanAddr> {
        bucket(storage, PREFIX_GATEWAYS_OWNERS_OLD)
    }

    fn mixnodes_old(storage: &mut dyn Storage) -> Bucket<MixNodeBond> {
        bucket(storage, PREFIX_MIXNODES_OLD)
    }

    fn gateways_old(storage: &mut dyn Storage) -> Bucket<GatewayBond> {
        bucket(storage, PREFIX_GATEWAYS_OLD)
    }

    // go through all stored mixnodes
    let bond_page_limit = 100;
    loop {
        // we have to do it in a paged manner to prevent allocating too much memory
        let nodes = mixnodes_old(deps.storage)
            .range(None, None, Order::Ascending)
            .take(bond_page_limit)
            .map(|res| res.map(|item| item.1))
            .collect::<StdResult<Vec<MixNodeBond>>>()?;

        for node in nodes.iter() {
            // save bond data under identity key
            mixnodes(deps.storage).save(node.identity().as_bytes(), node)?;

            // and remove it from under the old owner key
            mixnodes_old(deps.storage).remove(node.owner.as_bytes());

            // add new mixnodes_owners data under owner key
            mixnodes_owners(deps.storage).save(node.owner.as_bytes(), node.identity())?;

            // and remove it from under the old identity key
            mixnodes_owners_old(deps.storage).remove(node.identity().as_bytes())
        }

        // this was the last page
        if nodes.len() < bond_page_limit {
            break;
        }
    }

    // repeat the procedure for gateways
    loop {
        // we have to do it in a paged manner to prevent allocating too much memory
        let nodes = gateways_old(deps.storage)
            .range(None, None, Order::Ascending)
            .take(bond_page_limit)
            .map(|res| res.map(|item| item.1))
            .collect::<StdResult<Vec<GatewayBond>>>()?;

        for node in nodes.iter() {
            // save bond data under identity key
            gateways(deps.storage).save(node.identity().as_bytes(), node)?;

            // and remove it from under the old owner key
            gateways_old(deps.storage).remove(node.owner.as_bytes());

            // add new mixnodes_owners data under owner key
            gateways_owners(deps.storage).save(node.owner.as_bytes(), node.identity())?;

            // and remove it from under the old identity key
            gateways_owners_old(deps.storage).remove(node.identity().as_bytes())
        }

        // this was the last page
        if nodes.len() < bond_page_limit {
            break;
        }
    }

    Ok(Default::default())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::support::tests::helpers::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};
    use mixnet_contract::PagedResponse;

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let msg = InitMsg {};
        let info = mock_info("creator", &[]);

        let res = init(deps.as_mut(), env.clone(), info, msg).unwrap();
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
        let page: PagedResponse = from_binary(&res).unwrap();
        assert_eq!(0, page.nodes.len()); // there are no mixnodes in the list when it's just been initialized

        // Contract balance should match what we initialized it as
        assert_eq!(
            coins(0, DENOM),
            query_contract_balance(env.contract.address, deps)
        );
    }

    // TODO: this test will have to be removed once the migration happens and we are working on yet another
    // version of the contract
    #[test]
    fn migration_to_layer_distribution() {
        //
    }
}
