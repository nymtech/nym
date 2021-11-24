// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet_contract_settings::models::ContractSettings;
use cosmwasm_std::StdResult;
use cosmwasm_std::Storage;
use cosmwasm_storage::singleton;
use cosmwasm_storage::singleton_read;
use cosmwasm_storage::ReadonlySingleton;
use cosmwasm_storage::Singleton;
use mixnet_contract::ContractSettingsParams;
use mixnet_contract::Layer;
use mixnet_contract::LayerDistribution;

// storage prefixes
const CONFIG_KEY: &[u8] = b"config";
const LAYER_DISTRIBUTION_KEY: &[u8] = b"layers";

pub fn contract_settings(storage: &mut dyn Storage) -> Singleton<ContractSettings> {
    singleton(storage, CONFIG_KEY)
}

pub fn contract_settings_read(storage: &dyn Storage) -> ReadonlySingleton<ContractSettings> {
    singleton_read(storage, CONFIG_KEY)
}

pub(crate) fn read_contract_settings_params(storage: &dyn Storage) -> ContractSettingsParams {
    // note: In any other case, I wouldn't have attempted to unwrap this result, but in here
    // if we fail to load the stored state we would already be in the undefined behaviour land,
    // so we better just blow up immediately.
    contract_settings_read(storage).load().unwrap().params
}

pub fn layer_distribution(storage: &mut dyn Storage) -> Singleton<LayerDistribution> {
    singleton(storage, LAYER_DISTRIBUTION_KEY)
}

pub(crate) fn read_layer_distribution(storage: &dyn Storage) -> LayerDistribution {
    // note: In any other case, I wouldn't have attempted to unwrap this result, but in here
    // if we fail to load the stored state we would already be in the undefined behaviour land,
    // so we better just blow up immediately.
    layer_distribution_read(storage).load().unwrap()
}

pub fn layer_distribution_read(storage: &dyn Storage) -> ReadonlySingleton<LayerDistribution> {
    singleton_read(storage, LAYER_DISTRIBUTION_KEY)
}

pub fn increment_layer_count(storage: &mut dyn Storage, layer: Layer) -> StdResult<()> {
    let mut distribution = layer_distribution(storage).load()?;
    match layer {
        Layer::Gateway => distribution.gateways += 1,
        Layer::One => distribution.layer1 += 1,
        Layer::Two => distribution.layer2 += 1,
        Layer::Three => distribution.layer3 += 1,
    }
    layer_distribution(storage).save(&distribution)
}

pub fn decrement_layer_count(storage: &mut dyn Storage, layer: Layer) -> StdResult<()> {
    let mut distribution = layer_distribution(storage).load()?;
    // It can't possibly go below zero, if it does, it means there's a serious error in the contract logic
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
    };
    layer_distribution(storage).save(&distribution)
}
