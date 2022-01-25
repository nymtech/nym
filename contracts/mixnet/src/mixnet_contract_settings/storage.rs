// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet_contract_settings::models::ContractState;
use cosmwasm_std::StdResult;
use cosmwasm_std::Storage;
use cw_storage_plus::Item;
use mixnet_contract_common::{Layer, LayerDistribution};

pub(crate) const CONTRACT_STATE: Item<ContractState> = Item::new("config");
pub(crate) const LAYERS: Item<LayerDistribution> = Item::new("layers");

pub fn increment_layer_count(storage: &mut dyn Storage, layer: Layer) -> StdResult<()> {
    LAYERS
        .update(storage, |mut distribution| {
            match layer {
                Layer::Gateway => distribution.gateways += 1,
                Layer::One => distribution.layer1 += 1,
                Layer::Two => distribution.layer2 += 1,
                Layer::Three => distribution.layer3 += 1,
            }
            Ok(distribution)
        })
        .map(|_| ())
}

pub fn decrement_layer_count(storage: &mut dyn Storage, layer: Layer) -> StdResult<()> {
    LAYERS
        .update(storage, |mut distribution| {
            match layer {
                Layer::Gateway => {
                    distribution.gateways = distribution
                        .gateways
                        .checked_sub(1)
                        .expect("tried to subtract from unsigned zero!")
                }
                Layer::One => {
                    distribution.layer1 = distribution
                        .layer1
                        .checked_sub(1)
                        .expect("tried to subtract from unsigned zero!")
                }
                Layer::Two => {
                    distribution.layer2 = distribution
                        .layer2
                        .checked_sub(1)
                        .expect("tried to subtract from unsigned zero!")
                }
                Layer::Three => {
                    distribution.layer3 = distribution
                        .layer3
                        .checked_sub(1)
                        .expect("tried to subtract from unsigned zero!")
                }
            }
            Ok(distribution)
        })
        .map(|_| ())
}
