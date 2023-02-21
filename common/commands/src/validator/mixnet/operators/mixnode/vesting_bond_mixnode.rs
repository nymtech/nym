// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use cosmwasm_std::Uint128;
use log::{info, warn};
use nym_mixnet_contract_common::{Coin, MixNodeCostParams};
use nym_mixnet_contract_common::{MixNode, Percent};
use nym_network_defaults::{
    DEFAULT_HTTP_API_LISTENING_PORT, DEFAULT_MIX_LISTENING_PORT, DEFAULT_VERLOC_LISTENING_PORT,
};
use validator_client::nyxd::{CosmWasmCoin, VestingSigningClient};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub host: String,

    #[clap(long)]
    pub signature: String,

    #[clap(long)]
    pub mix_port: Option<u16>,

    #[clap(long)]
    pub verloc_port: Option<u16>,

    #[clap(long)]
    pub http_api_port: Option<u16>,

    #[clap(long)]
    pub sphinx_key: String,

    #[clap(long)]
    pub identity_key: String,

    #[clap(long)]
    pub version: String,

    #[clap(long)]
    pub profit_margin_percent: Option<u8>,

    #[clap(
        long,
        help = "operating cost in current DENOMINATION (so it would be 'unym', rather than 'nym')"
    )]
    pub interval_operating_cost: Option<u128>,

    #[clap(
        long,
        help = "bonding amount in current DENOMINATION (so it would be 'unym', rather than 'nym')"
    )]
    pub amount: u128,

    #[clap(long)]
    pub gas: Option<u64>,

    #[clap(short, long)]
    pub force: bool,
}

pub async fn vesting_bond_mixnode(client: SigningClient, args: Args, denom: &str) {
    info!("Starting vesting mixnode bonding!");

    // if we're trying to bond less than 1 token
    if args.amount < 1_000_000 && !args.force {
        warn!("You're trying to bond only {}{} which is less than 1 full token. Are you sure that's what you want? If so, run with `--force` or `-f` flag", args.amount, denom);
        return;
    }

    let mixnode = MixNode {
        host: args.host,
        mix_port: args.mix_port.unwrap_or(DEFAULT_MIX_LISTENING_PORT),
        verloc_port: args.verloc_port.unwrap_or(DEFAULT_VERLOC_LISTENING_PORT),
        http_api_port: args
            .http_api_port
            .unwrap_or(DEFAULT_HTTP_API_LISTENING_PORT),
        sphinx_key: args.sphinx_key,
        identity_key: args.identity_key,
        version: args.version,
    };

    let coin = Coin::new(args.amount, denom);

    let cost_params = MixNodeCostParams {
        profit_margin_percent: Percent::from_percentage_value(
            args.profit_margin_percent.unwrap_or(10) as u64,
        )
        .unwrap(),
        interval_operating_cost: CosmWasmCoin {
            denom: denom.into(),
            amount: Uint128::new(args.interval_operating_cost.unwrap_or(40_000_000)),
        },
    };

    let res = client
        .vesting_bond_mixnode(mixnode, cost_params, &args.signature, coin.into(), None)
        .await
        .expect("failed to bond vesting mixnode!");

    info!("Bonding vesting result: {:?}", res)
}
