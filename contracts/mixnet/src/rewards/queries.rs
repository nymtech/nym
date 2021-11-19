use crate::storage::circulating_supply;
use crate::storage::reward_pool_value;
use cosmwasm_std::Deps;
use cosmwasm_std::Uint128;

pub(crate) fn query_reward_pool(deps: Deps) -> Uint128 {
    reward_pool_value(deps.storage)
}

pub(crate) fn query_circulating_supply(deps: Deps) -> Uint128 {
    circulating_supply(deps.storage)
}
