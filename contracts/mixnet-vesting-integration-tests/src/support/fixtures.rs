// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::support::setup::{MIX_DENOM, REWARDING_VALIDATOR};
use cosmwasm_std::Decimal;
use nym_contracts_common::Percent;
use nym_mixnet_contract_common::InitialRewardingParams;
use std::time::Duration;

pub fn default_mixnet_init_msg() -> nym_mixnet_contract_common::InstantiateMsg {
    nym_mixnet_contract_common::InstantiateMsg {
        rewarding_validator_address: REWARDING_VALIDATOR.to_string(),
        vesting_contract_address: "placeholder".to_string(),
        rewarding_denom: MIX_DENOM.to_string(),
        epochs_in_interval: 720,
        epoch_duration: Duration::from_secs(60 * 60),
        initial_rewarding_params: InitialRewardingParams {
            initial_reward_pool: Decimal::from_atomics(250_000_000_000_000u128, 0).unwrap(),
            initial_staking_supply: Decimal::from_atomics(223_000_000_000_000u128, 0).unwrap(),
            staking_supply_scale_factor: Percent::hundred(),
            sybil_resistance: Percent::from_percentage_value(30).unwrap(),
            active_set_work_factor: Decimal::from_atomics(10u32, 0).unwrap(),
            interval_pool_emission: Percent::from_percentage_value(2).unwrap(),
            rewarded_set_size: 240,
            active_set_size: 100,
        },
        profit_margin: Default::default(),
        interval_operating_cost: Default::default(),
    }
}
