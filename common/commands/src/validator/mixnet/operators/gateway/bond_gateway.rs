// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::{info, warn};
use nym_contracts_common::signing::MessageSignature;
use nym_mixnet_contract_common::Coin;
use nym_network_defaults::{DEFAULT_CLIENT_LISTENING_PORT, DEFAULT_MIX_LISTENING_PORT};
use nym_validator_client::nyxd::contract_traits::MixnetSigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub host: String,

    #[clap(long)]
    pub signature: MessageSignature,

    #[clap(long)]
    pub mix_port: Option<u16>,

    #[clap(long)]
    pub clients_port: Option<u16>,

    #[clap(long)]
    pub location: String,

    #[clap(long)]
    pub sphinx_key: String,

    #[clap(long)]
    pub identity_key: String,

    #[clap(long)]
    pub version: String,

    #[clap(
        long,
        help = "bonding amount in current DENOMINATION (so it would be 'unym', rather than 'nym')"
    )]
    pub amount: u128,

    #[clap(short, long)]
    pub force: bool,
}

pub async fn bond_gateway(args: Args, client: SigningClient) {
    let denom = client.current_chain_details().mix_denom.base.as_str();

    info!("Starting gateway bonding!");

    // if we're trying to bond less than 1 token
    if args.amount < 1_000_000 && !args.force {
        warn!("You're trying to bond only {}{} which is less than 1 full token. Are you sure that's what you want? If so, run with `--force` or `-f` flag", args.amount, denom);
        return;
    }

    let gateway = nym_mixnet_contract_common::Gateway {
        host: args.host,
        mix_port: args.mix_port.unwrap_or(DEFAULT_MIX_LISTENING_PORT),
        clients_port: args.clients_port.unwrap_or(DEFAULT_CLIENT_LISTENING_PORT),
        location: args.location,
        sphinx_key: args.sphinx_key,
        identity_key: args.identity_key,
        version: args.version,
    };

    let coin = Coin::new(args.amount, denom);

    let res = client
        .bond_gateway(gateway, args.signature, coin.into(), None)
        .await
        .expect("failed to bond gateway!");

    info!("Bonding result: {:?}", res)
}
