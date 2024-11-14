// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use cosmwasm_std::Decimal;
use log::{debug, info};
use nym_mixnet_contract_common::reward_params::RewardedSetParams;
use nym_mixnet_contract_common::{
    InitialRewardingParams, InstantiateMsg, OperatingCostRange, Percent, ProfitMarginRange,
};
use nym_network_defaults::mainnet::MIX_DENOM;
use nym_network_defaults::TOTAL_SUPPLY;
use nym_validator_client::nyxd::{AccountId, Coin};
use std::str::FromStr;
use std::time::Duration;

pub fn default_maximum_operating_cost() -> Coin {
    Coin::new(TOTAL_SUPPLY, MIX_DENOM.base)
}

pub fn default_minimum_operating_cost() -> Coin {
    Coin::new(0, MIX_DENOM.base)
}

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub rewarding_validator_address: Option<AccountId>,

    #[clap(long)]
    pub vesting_contract_address: Option<AccountId>,

    #[clap(long)]
    pub rewarding_denom: Option<String>,

    #[clap(long)]
    pub current_nym_node_version: String,

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

    #[clap(long, default_value_t = 50)]
    pub(crate) entry_gateways: u32,

    #[clap(long, default_value_t = 70)]
    pub(crate) exit_gateways: u32,

    #[clap(long, default_value_t = 120)]
    pub(crate) mixnodes: u32,

    #[clap(long, default_value_t = 0)]
    pub(crate) standby: u32,

    #[clap(long, default_value_t = Percent::zero())]
    pub minimum_profit_margin_percent: Percent,

    #[clap(long, default_value_t = Percent::hundred())]
    pub maximum_profit_margin_percent: Percent,

    #[clap(long, default_value_t = default_minimum_operating_cost())]
    pub minimum_interval_operating_cost: Coin,

    #[clap(long, default_value_t = default_maximum_operating_cost())]
    pub maximum_interval_operating_cost: Coin,
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

        rewarded_set_params: RewardedSetParams {
            entry_gateways: args.entry_gateways,
            exit_gateways: args.exit_gateways,
            mixnodes: args.mixnodes,
            standby: args.standby,
        },
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

    if args.minimum_interval_operating_cost.denom != args.maximum_interval_operating_cost.denom {
        panic!("different denoms for operating cost bounds")
    }

    let instantiate_msg = InstantiateMsg {
        rewarding_validator_address: rewarding_validator_address.to_string(),
        vesting_contract_address: vesting_contract_address.to_string(),
        rewarding_denom,
        epochs_in_interval: args.epochs_in_interval,
        epoch_duration: Duration::from_secs(args.epoch_duration),
        initial_rewarding_params,
        current_nym_node_version: args.current_nym_node_version,
        version_score_weights: Default::default(),
        version_score_params: Default::default(),
        profit_margin: ProfitMarginRange {
            minimum: args.minimum_profit_margin_percent,
            maximum: args.maximum_profit_margin_percent,
        },
        interval_operating_cost: OperatingCostRange {
            minimum: args.minimum_interval_operating_cost.amount.into(),
            maximum: args.maximum_interval_operating_cost.amount.into(),
        },
    };

    debug!("instantiate_msg: {:?}", instantiate_msg);

    let res =
        serde_json::to_string(&instantiate_msg).expect("failed to convert instantiate msg to json");

    println!("{res}")
}
