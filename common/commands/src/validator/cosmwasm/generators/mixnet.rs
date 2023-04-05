// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::{debug, info};

use cosmwasm_std::Decimal;
use nym_mixnet_contract_common::{InitialRewardingParams, InstantiateMsg, Percent};
use std::str::FromStr;
use std::time::Duration;
use nym_validator_client::nyxd::AccountId;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub rewarding_validator_address: Option<AccountId>,

    #[clap(long)]
    pub vesting_contract_address: Option<AccountId>,

    #[clap(long)]
    pub rewarding_denom: Option<String>,

    #[clap(long, default_value_t = 720)]
    pub epochs_in_interval: u32,

    #[clap(long, default_value_t = 60*60)]
    pub epoch_duration: u64,

    #[clap(long, default_value_t = 244_817_525_850_285)]
    pub initial_reward_pool: u128,

    #[clap(long, default_value_t = 100_000_000_000_000)]
    pub initial_staking_supply: u128,

    #[clap(long, default_value_t = 50)]
    pub staking_supply_scale_factor: u64,

    #[clap(long, default_value_t = 30)]
    pub sybil_resistance: u64,

    #[clap(long, default_value_t = 10)]
    pub active_set_work_factor: u32,

    #[clap(long, default_value_t = 2)]
    pub interval_pool_emission: u64,

    #[clap(long, default_value_t = 240)]
    pub rewarded_set_size: u32,

    #[clap(long, default_value_t = 240)]
    pub active_set_size: u32,
}

pub async fn generate(args: Args) {
    info!("Starting to generate mixnet contract instantiate msg");

    debug!("Received arguments: {:?}", args);

    let initial_rewarding_params = InitialRewardingParams {
        initial_reward_pool: Decimal::from_atomics(args.initial_reward_pool, 0)
            .expect("initial_rewarding_pool can't be converted to Decimal"),
        initial_staking_supply: Decimal::from_atomics(args.initial_staking_supply, 0)
            .expect("initial_staking_supply can't be converted to Decimal"),
        staking_supply_scale_factor: Percent::from_percentage_value(
            args.staking_supply_scale_factor,
        )
        .unwrap(),
        sybil_resistance: Percent::from_percentage_value(args.sybil_resistance)
            .expect("sybil_resistance can't be converted to Percent"),
        active_set_work_factor: Decimal::from_atomics(args.active_set_work_factor, 0)
            .expect("active_set_work_factor can't be converted to Decimal"),
        interval_pool_emission: Percent::from_percentage_value(args.interval_pool_emission)
            .expect("interval_pool_emission can't be converted to Percent"),
        rewarded_set_size: args.rewarded_set_size,
        active_set_size: args.active_set_size,
    };

    debug!("initial_rewarding_params: {:?}", initial_rewarding_params);

    let rewarding_validator_address = args.rewarding_validator_address.unwrap_or_else(|| {
        let address = std::env::var(nym_network_defaults::var_names::REWARDING_VALIDATOR_ADDRESS)
            .expect("Rewarding validator address has to be set");
        AccountId::from_str(address.as_str())
            .expect("Failed converting rewarding validator address to AccountId")
    });

    let vesting_contract_address = args.vesting_contract_address.unwrap_or_else(|| {
        let address = std::env::var(nym_network_defaults::var_names::VESTING_CONTRACT_ADDRESS)
            .expect("Vesting contract address has to be set");
        AccountId::from_str(address.as_str())
            .expect("Failed converting vesting contract address to AccountId")
    });

    let rewarding_denom = args.rewarding_denom.unwrap_or_else(|| {
        std::env::var(nym_network_defaults::var_names::MIX_DENOM)
            .expect("Rewarding (mix) denom has to be set")
    });

    let instantiate_msg = InstantiateMsg {
        rewarding_validator_address: rewarding_validator_address.to_string(),
        vesting_contract_address: vesting_contract_address.to_string(),
        rewarding_denom,
        epochs_in_interval: args.epochs_in_interval,
        epoch_duration: Duration::from_secs(args.epoch_duration),
        initial_rewarding_params,
    };

    debug!("instantiate_msg: {:?}", instantiate_msg);

    let res =
        serde_json::to_string(&instantiate_msg).expect("failed to convert instantiate msg to json");

    println!("{res}")
}
