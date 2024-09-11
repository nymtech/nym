// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use crate::utils::{account_id_to_cw_addr, DataWrapper};
use clap::Parser;
use cosmwasm_std::{Coin, Uint128};
use nym_bin_common::output_format::OutputFormat;
use nym_contracts_common::Percent;
use nym_mixnet_contract_common::{
    construct_legacy_mixnode_bonding_sign_payload, MixNodeCostParams,
};
use nym_network_defaults::{
    DEFAULT_HTTP_API_LISTENING_PORT, DEFAULT_MIX_LISTENING_PORT, DEFAULT_VERLOC_LISTENING_PORT,
};
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use nym_validator_client::nyxd::CosmWasmCoin;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub host: String,

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

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub async fn create_payload(args: Args, client: SigningClient) {
    let denom = client.current_chain_details().mix_denom.base.as_str();

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

    let nonce = match client.get_signing_nonce(&client.address()).await {
        Ok(nonce) => nonce,
        Err(err) => {
            eprint!(
                "failed to query for the signing nonce of {}: {err}",
                client.address()
            );
            return;
        }
    };

    let address = account_id_to_cw_addr(&client.address());

    let payload =
        construct_legacy_mixnode_bonding_sign_payload(nonce, address, coin, mixnode, cost_params);
    let wrapper = DataWrapper::new(payload.to_base58_string().unwrap());
    println!("{}", args.output.format(&wrapper))
}
