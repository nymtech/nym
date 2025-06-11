// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in test code
#![allow(clippy::unwrap_used)]

use crate::contract::{execute, instantiate, migrate, query};
use cosmwasm_std::Decimal;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::reward_params::RewardedSetParams;
use mixnet_contract_common::{
    ExecuteMsg, InitialRewardingParams, InstantiateMsg, MigrateMsg, QueryMsg,
};
use nym_contracts_common::Percent;
use nym_contracts_common_testing::{
    mock_dependencies, ContractFn, PermissionedFn, QueryFn, TEST_DENOM,
};
use std::time::Duration;

pub use nym_contracts_common_testing::TestableNymContract;

pub struct MixnetContract;

fn initial_rewarded_set_params() -> RewardedSetParams {
    RewardedSetParams {
        entry_gateways: 50,
        exit_gateways: 70,
        mixnodes: 120,
        standby: 50,
    }
}

fn initial_rewarding_params() -> InitialRewardingParams {
    let reward_pool = 250_000_000_000_000u128;
    let staking_supply = 100_000_000_000_000u128;

    InitialRewardingParams {
        initial_reward_pool: Decimal::from_atomics(reward_pool, 0).unwrap(), // 250M * 1M (we're expressing it all in base tokens)
        initial_staking_supply: Decimal::from_atomics(staking_supply, 0).unwrap(), // 100M * 1M
        staking_supply_scale_factor: Percent::hundred(),
        sybil_resistance: Percent::from_percentage_value(30).unwrap(),
        active_set_work_factor: Decimal::from_atomics(10u32, 0).unwrap(),
        interval_pool_emission: Percent::from_percentage_value(2).unwrap(),
        rewarded_set_params: initial_rewarded_set_params(),
    }
}

impl TestableNymContract for MixnetContract {
    const NAME: &'static str = "mixnet-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = MixnetContractError;

    fn instantiate() -> ContractFn<Self::InitMsg, Self::ContractError> {
        instantiate
    }

    fn execute() -> ContractFn<Self::ExecuteMsg, Self::ContractError> {
        execute
    }

    fn query() -> QueryFn<Self::QueryMsg, Self::ContractError> {
        query
    }

    fn migrate() -> PermissionedFn<Self::MigrateMsg, Self::ContractError> {
        migrate
    }

    fn base_init_msg() -> Self::InitMsg {
        let deps = mock_dependencies();
        InstantiateMsg {
            rewarding_validator_address: deps.api.addr_make("rewarder").to_string(),
            vesting_contract_address: deps.api.addr_make("vesting-contract").to_string(),
            rewarding_denom: TEST_DENOM.to_string(),
            epochs_in_interval: 720,
            epoch_duration: Duration::from_secs(60 * 60),
            initial_rewarding_params: initial_rewarding_params(),
            current_nym_node_version: "1.1.10".to_string(),
            version_score_weights: Default::default(),
            version_score_params: Default::default(),
            profit_margin: Default::default(),
            interval_operating_cost: Default::default(),
            key_validity_in_epochs: None,
        }
    }
}
