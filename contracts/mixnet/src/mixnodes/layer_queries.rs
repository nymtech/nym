use crate::storage;
use cosmwasm_std::Deps;
use mixnet_contract::LayerDistribution;

pub(crate) fn query_layer_distribution(deps: Deps) -> LayerDistribution {
    storage::read_layer_distribution(deps.storage)
}
