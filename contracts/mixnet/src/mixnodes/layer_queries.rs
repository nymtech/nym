// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use cosmwasm_std::Deps;
use mixnet_contract::LayerDistribution;

pub(crate) fn query_layer_distribution(deps: Deps) -> LayerDistribution {
    mixnet_params_storage::read_layer_distribution(deps.storage)
}
