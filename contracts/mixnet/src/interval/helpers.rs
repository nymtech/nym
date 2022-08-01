// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interval::storage;
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{Response, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::new_interval_config_update_event;
use mixnet_contract_common::Interval;
use std::time::Duration;

pub(crate) fn change_interval_config(
    store: &mut dyn Storage,
    mut current_interval: Interval,
    epochs_in_interval: u32,
    epoch_duration_secs: u64,
) -> Result<Response, MixnetContractError> {
    current_interval.change_epoch_length(Duration::from_secs(epoch_duration_secs));

    let mut rewarding_params = rewards_storage::REWARDING_PARAMS.load(store)?;
    rewarding_params.apply_epochs_in_interval_change(epochs_in_interval);
    rewards_storage::REWARDING_PARAMS.save(store, &rewarding_params)?;

    current_interval.force_change_epochs_in_interval(epochs_in_interval);
    storage::save_interval(store, &current_interval)?;

    Ok(Response::new().add_event(new_interval_config_update_event(
        epochs_in_interval,
        epoch_duration_secs,
        rewarding_params.interval,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::Decimal;

    #[test]
    fn changing_interval_config() {
        let two = Decimal::from_atomics(2u32, 0).unwrap();
        let mut deps = test_helpers::init_contract();

        let initial_interval = storage::current_interval(&deps.storage).unwrap();
        let initial_params = rewards_storage::REWARDING_PARAMS
            .load(&deps.storage)
            .unwrap();

        // if we half the number of epochs, the reward budget should get doubled
        change_interval_config(
            &mut deps.storage,
            initial_interval,
            initial_interval.epochs_in_interval() / 2,
            initial_interval.epoch_length_secs(),
        )
        .unwrap();
        let updated_interval = storage::current_interval(&deps.storage).unwrap();
        let updated_params = rewards_storage::REWARDING_PARAMS
            .load(&deps.storage)
            .unwrap();

        assert_eq!(
            updated_interval.epochs_in_interval(),
            initial_interval.epochs_in_interval() / 2
        );
        assert_eq!(
            updated_params.interval.epoch_reward_budget,
            initial_params.interval.epoch_reward_budget * two
        );

        // and similarly when we double number of epochs, the reward budget should get halved
        change_interval_config(
            &mut deps.storage,
            initial_interval,
            initial_interval.epochs_in_interval() * 2,
            initial_interval.epoch_length_secs(),
        )
        .unwrap();

        // if we half the number of epochs, the reward budget should get doubled
        let updated_interval = storage::current_interval(&deps.storage).unwrap();
        let updated_params = rewards_storage::REWARDING_PARAMS
            .load(&deps.storage)
            .unwrap();

        assert_eq!(
            updated_interval.epochs_in_interval(),
            initial_interval.epochs_in_interval() * 2
        );
        assert_eq!(
            updated_params.interval.epoch_reward_budget,
            initial_params.interval.epoch_reward_budget / two
        );
    }
}
