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

// for time being completely ignore concept of a leap year and assume each year is exactly 365 days
// i.e. 8760 hours
const HOURS_IN_YEAR: u128 = 8760;

// annoyingly not exposed by `Decimal` directly.
const DECIMAL_FRACTIONAL: Uint128 = Uint128(1_000_000_000_000_000_000u128);

// calculates value - 1
fn decimal_sub_one(value: Decimal) -> Decimal {
    assert!(value >= Decimal::one());

    // those conversions are so freaking disgusting and I fear they might result in some loss of precision
    let value_uint128 = value * DECIMAL_FRACTIONAL;
    let uint128_sub_one = (value_uint128 - DECIMAL_FRACTIONAL).unwrap();
    Decimal::from_ratio(uint128_sub_one, DECIMAL_FRACTIONAL)
}

// I don't like this, but this seems to be the only way of converting Decimal into Uint128
fn decimal_to_uint128(value: Decimal) -> Uint128 {
    // TODO: This function should have some proper bound checks implemented to ensure no overflow
    value * DECIMAL_FRACTIONAL
}

// another disgusting conversion, assumes `value` was already multiplied by `DECIMAL_FRACTIONAL` before
fn uint128_to_decimal(value: Uint128) -> Decimal {
    Decimal::from_ratio(value, uint128_decimal_one())
}

const fn uint128_decimal_one() -> Uint128 {
    DECIMAL_FRACTIONAL
}

// TODO: this does not seem fully right, I'm not sure what that is exactly,
// but it feels like something is not taken into consideration,
// like compound interest BS or some other exponentiation stuff
fn calculate_epoch_reward_rate(epoch_length: u32, annual_reward_rate: Decimal) -> Decimal {
    // this is more of a sanity check as the contract does not allow setting annual reward rates
    // to be lower than 1.
    debug_assert!(annual_reward_rate >= Decimal::one());

    // converts reward rate, like 1.25 into the expected gain, like 0.25
    let annual_reward = decimal_sub_one(annual_reward_rate);
    // do a simple cross-multiplication:
    // `annual_reward`  -    `HOURS_IN_YEAR`
    //          x       -    `epoch_length`
    //
    // x = `annual_reward` * `epoch_length` / `HOURS_IN_YEAR`

    let epoch_ratio = Decimal::from_ratio(epoch_length, HOURS_IN_YEAR);

    // converts reward, like 0.25 into 1250000000000000000
    let annual_reward_uint128 = decimal_to_uint128(annual_reward);

    let epoch_reward_uint128 = epoch_ratio * annual_reward_uint128;
    let epoch_reward = uint128_to_decimal(epoch_reward_uint128);

    // convert back into the reward rate, i.e. 0.25 into 1.25
    epoch_reward + Decimal::one()
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

    #[test]
    fn calculating_epoch_reward_rate() {
        // 1.10
        let annual_reward_rate = Decimal::from_ratio(110u128, 100u128);
        let annual_reward = decimal_sub_one(annual_reward_rate);

        // if the epoch is (for some reason) exactly one year,
        // the reward rate should be unchanged
        let per_epoch_rate = calculate_epoch_reward_rate(HOURS_IN_YEAR as u32, annual_reward_rate);
        assert_eq!(per_epoch_rate, annual_reward_rate);

        // 24 hours
        let per_epoch_rate = calculate_epoch_reward_rate(24, annual_reward_rate);

        // 0.1 / 365
        let expected_epoch_reward = Decimal::from_ratio(1u128, 3650u128);

        // 0.1 / 365 + 1
        let expected_epoch_reward_rate =
            decimal_to_uint128(expected_epoch_reward) + uint128_decimal_one();
        let expected = uint128_to_decimal(expected_epoch_reward_rate);

        assert_eq!(expected, per_epoch_rate);

        // 1 hour
        let per_epoch_rate = calculate_epoch_reward_rate(1, annual_reward_rate);

        // 0.1 / 8760
        let expected_epoch_reward = Decimal::from_ratio(1u128, 87600u128);

        // 0.1 / 87600 + 1
        let expected_epoch_reward_rate =
            decimal_to_uint128(expected_epoch_reward) + uint128_decimal_one();
        let expected = uint128_to_decimal(expected_epoch_reward_rate);

        assert_eq!(expected, per_epoch_rate);
    }
}
