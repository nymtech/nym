use crate::mixnet_params::storage as mixnet_params_storage;
use cosmwasm_std::Deps;
use cosmwasm_std::Uint128;

pub(crate) fn query_reward_pool(deps: Deps) -> Uint128 {
    mixnet_params_storage::reward_pool_value(deps.storage)
}

pub(crate) fn query_circulating_supply(deps: Deps) -> Uint128 {
    mixnet_params_storage::circulating_supply(deps.storage)
}
