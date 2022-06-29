// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnodes::storage;
use cosmwasm_std::{Deps, StdResult};
use mixnet_contract_common::LayerDistribution;

pub(crate) fn query_layer_distribution(deps: Deps<'_>) -> StdResult<LayerDistribution> {
    storage::LAYERS.load(deps.storage)
}
