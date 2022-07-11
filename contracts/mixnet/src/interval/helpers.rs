// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interval::storage;
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::Storage;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::Interval;

pub(crate) fn change_epochs_in_interval(
    store: &mut dyn Storage,
    current_interval: Option<Interval>,
    epochs_in_interval: u32,
) -> Result<(), MixnetContractError> {
    let mut interval = match current_interval {
        Some(interval) => interval,
        None => storage::current_interval(store)?,
    };

    let mut rewarding_params = rewards_storage::REWARDING_PARAMS.load(store)?;
    rewarding_params.apply_epochs_in_interval_change(epochs_in_interval);
    rewards_storage::REWARDING_PARAMS.save(store, &rewarding_params)?;

    interval.force_change_epochs_in_interval(epochs_in_interval);
    storage::save_interval(store, &interval)
}
