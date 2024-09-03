// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use crate::utils::{account_id_to_cw_addr, DataWrapper};
use clap::Parser;
use cosmwasm_std::Coin;
use nym_bin_common::output_format::OutputFormat;
use nym_mixnet_contract_common::construct_legacy_gateway_bonding_sign_payload;
use nym_network_defaults::{DEFAULT_CLIENT_LISTENING_PORT, DEFAULT_MIX_LISTENING_PORT};
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub host: String,

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

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub async fn create_payload(args: Args, client: SigningClient) {
    let denom = client.current_chain_details().mix_denom.base.as_str();

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

    let payload = construct_legacy_gateway_bonding_sign_payload(nonce, address, coin, gateway);
    let wrapper = DataWrapper::new(payload.to_base58_string().unwrap());
    println!("{}", args.output.format(&wrapper))
}
