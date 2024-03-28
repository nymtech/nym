// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use cosmwasm_std::Uint128;
use log::{info, warn};
use nym_contracts_common::signing::MessageSignature;
use nym_mixnet_contract_common::{
    Coin, NodeCostParams, Percent, DEFAULT_INTERVAL_OPERATING_COST_AMOUNT,
    DEFAULT_PROFIT_MARGIN_PERCENT,
};
use nym_network_defaults::{
    DEFAULT_HTTP_API_LISTENING_PORT, DEFAULT_MIX_LISTENING_PORT, DEFAULT_VERLOC_LISTENING_PORT,
};
use nym_validator_client::nyxd::contract_traits::MixnetSigningClient;
use nym_validator_client::nyxd::CosmWasmCoin;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub host: String,

    #[clap(long)]
    pub signature: MessageSignature,

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
    pub profit_margin_percent: Option<u64>,

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

    #[clap(short, long)]
    pub force: bool,
}

pub async fn bond_mixnode(args: Args, client: SigningClient) {
    let denom = client.current_chain_details().mix_denom.base.as_str();

    info!("Starting mixnode bonding!");

    // if we're trying to bond less than 1 token
    if args.amount < 1_000_000 && !args.force {
        warn!("You're trying to bond only {}{} which is less than 1 full token. Are you sure that's what you want? If so, run with `--force` or `-f` flag", args.amount, denom);
        return;
    }

    let mixnode = nym_mixnet_contract_common::MixNode {
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

    let cost_params = NodeCostParams {
        profit_margin_percent: Percent::from_percentage_value(
            args.profit_margin_percent
                .unwrap_or(DEFAULT_PROFIT_MARGIN_PERCENT),
        )
        .unwrap(),
        interval_operating_cost: CosmWasmCoin {
            denom: denom.into(),
            amount: Uint128::new(
                args.interval_operating_cost
                    .unwrap_or(DEFAULT_INTERVAL_OPERATING_COST_AMOUNT),
            ),
        },
    };

    let res = client
        .bond_mixnode(mixnode, cost_params, args.signature, coin.into(), None)
        .await
        .expect("failed to bond mixnode!");

    info!("Bonding result: {:?}", res)
}
