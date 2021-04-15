use crate::msg::{HandleMsg, InitMsg, MigrateMsg, QueryMsg};
use crate::state::{config, State, StateParams};
use crate::{error::ContractError, queries, transactions};
use cosmwasm_std::{
    to_binary, Decimal, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    MigrateResponse, QueryResponse, Uint128,
};

/// Constant specifying minimum of coin required to bond a gateway
pub const INITIAL_GATEWAY_BOND: Uint128 = Uint128(100_000000);

/// Constant specifying minimum of coin required to bond a mixnode
pub const INITIAL_MIXNODE_BOND: Uint128 = Uint128(100_000000);

pub const INITIAL_MIXNODE_BOND_REWARD_RATE: Decimal = Decimal::one();

pub const INITIAL_GATEWAY_BOND_REWARD_RATE: Decimal = Decimal::one();

pub const INITIAL_MIXNODE_ACTIVE_SET_SIZE: u32 = 100;

/// Constant specifying denomination of the coin used for bonding
pub const DENOM: &str = "uhal";

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
    // TODO: to discuss with DH, should the initial state be set as it is right now, i.e.
    // using the defined constants, or should it rather be all based on whatever is sent
    // in `InitMsg`?
    let state = State {
        owner: info.sender,
        params: StateParams {
            minimum_mixnode_bond: INITIAL_MIXNODE_BOND,
            minimum_gateway_bond: INITIAL_GATEWAY_BOND,
            mixnode_bond_reward_rate: INITIAL_MIXNODE_BOND_REWARD_RATE,
            gateway_bond_reward_rate: INITIAL_GATEWAY_BOND_REWARD_RATE,
            mixnode_active_set_size: INITIAL_MIXNODE_ACTIVE_SET_SIZE,
        },
    };
    config(deps.storage).save(&state)?;
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
        HandleMsg::UnbondMixnode {} => transactions::try_remove_mixnode(deps, info, env),
        HandleMsg::BondGateway { gateway } => transactions::try_add_gateway(deps, info, gateway),
        HandleMsg::UnbondGateway {} => transactions::try_remove_gateway(deps, info, env),
        HandleMsg::UpdateStateParams(params) => {
            transactions::try_update_state_params(deps, info, params)
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
    };

    Ok(query_res?)
}

pub fn migrate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: MigrateMsg,
) -> Result<MigrateResponse, ContractError> {
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
}
