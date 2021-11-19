use crate::storage::read_layer_distribution;
use cosmwasm_std::Deps;
use mixnet_contract::LayerDistribution;

pub(crate) fn query_layer_distribution(deps: Deps) -> LayerDistribution {
    read_layer_distribution(deps.storage)
}
