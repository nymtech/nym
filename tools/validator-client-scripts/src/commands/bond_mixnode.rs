// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Client;
use clap::Parser;
use log::{info, warn};
use mixnet_contract_common::Coin;
use network_defaults::{
    DEFAULT_HTTP_API_LISTENING_PORT, DEFAULT_MIX_LISTENING_PORT, DEFAULT_VERLOC_LISTENING_PORT,
};

#[derive(Debug, Parser)]
pub(crate) struct Args {
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
        help = "bonding amount in current DENOMINATION (so it would be 'unym', rather than 'nym')"
    )]
    pub amount: u128,

    #[clap(long)]
    pub gas: Option<u64>,

    #[clap(short, long)]
    pub force: bool,
}

pub(crate) async fn bond_mixnode(client: Client, args: Args, denom: &str) {
    info!("Starting mixnode bonding!");

    // if we're trying to bond less than 1 token
    if args.amount < 1_000_000 && !args.force {
        warn!("You're trying to bond only {}{} which is less than 1 full token. Are you sure that's what you want? If so, run with `--force` or `-f` flag", args.amount, denom);
        return;
    }

    let mixnode = mixnet_contract_common::MixNode {
        host: args.host,
        mix_port: args.mix_port.unwrap_or(DEFAULT_MIX_LISTENING_PORT),
        verloc_port: args.verloc_port.unwrap_or(DEFAULT_VERLOC_LISTENING_PORT),
        http_api_port: args
            .http_api_port
            .unwrap_or(DEFAULT_HTTP_API_LISTENING_PORT),
        sphinx_key: args.sphinx_key,
        identity_key: args.identity_key,
        version: args.version,
        profit_margin_percent: args.profit_margin_percent.unwrap_or(10),
    };

    let coin = Coin::new(args.amount, denom);

    let res = client
        .bond_mixnode(mixnode, args.signature, coin.into(), None)
        .await
        .expect("failed to bond mixnode!");

    info!("Bonding result: {:?}", res)
}
