use crate::mixnet_params::storage as mixnet_params_storage;
use cosmwasm_std::Deps;
use mixnet_contract::LayerDistribution;

pub(crate) fn query_layer_distribution(deps: Deps) -> LayerDistribution {
    mixnet_params_storage::read_layer_distribution(deps.storage)
}
