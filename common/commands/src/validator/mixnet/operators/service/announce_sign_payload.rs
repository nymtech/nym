// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    context::SigningClient,
    utils::{account_id_to_cw_addr, DataWrapper},
};

use clap::Parser;
use cosmwasm_std::Coin;

use nym_bin_common::output_format::OutputFormat;
use nym_service_provider_directory_common::{
    signing_types::construct_service_provider_announce_sign_payload, NymAddress,
    ServiceType::NetworkRequester,
};
use nym_sphinx::addressing::clients::Recipient;
use nym_validator_client::nyxd::traits::MixnetQueryClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub nym_address: Recipient,

    #[clap(long)]
    pub amount: u128,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub async fn create_payload(args: Args, client: SigningClient) {
    let denom = client.current_chain_details().mix_denom.base.as_str();

    //let mixnode = nym_mixnet_contract_common::MixNode {
    //    host: args.host,
    //    mix_port: args.mix_port.unwrap_or(DEFAULT_MIX_LISTENING_PORT),
    //    verloc_port: args.verloc_port.unwrap_or(DEFAULT_VERLOC_LISTENING_PORT),
    //    http_api_port: args
    //        .http_api_port
    //        .unwrap_or(DEFAULT_HTTP_API_LISTENING_PORT),
    //    sphinx_key: args.sphinx_key,
    //    identity_key: args.identity_key,
    //    version: args.version,
    //};

    //let nym_address = NymAddress::try_from_base58_string(args.nym_address)
    //.expect("provided nym address is not a valid nym address");
    let service = nym_service_provider_directory_common::ServiceDetails {
        nym_address: NymAddress::new(&args.nym_address.to_string()),
        service_type: NetworkRequester,
    };

    let coin = Coin::new(args.amount, denom);

    //let cost_params = MixNodeCostParams {
    //    profit_margin_percent: Percent::from_percentage_value(
    //        args.profit_margin_percent.unwrap_or(10) as u64,
    //    )
    //    .unwrap(),
    //    interval_operating_cost: CosmWasmCoin {
    //        denom: denom.into(),
    //        amount: Uint128::new(args.interval_operating_cost.unwrap_or(40_000_000)),
    //    },
    //};

    let nonce = match client.get_signing_nonce(client.address()).await {
        Ok(nonce) => nonce,
        Err(err) => {
            eprint!(
                "failed to query for the signing nonce of {}: {err}",
                client.address()
            );
            return;
        }
    };

    let address = account_id_to_cw_addr(client.address());
    //let proxy = if args.with_vesting_account {
    //    Some(account_id_to_cw_addr(client.vesting_contract_address()))
    //} else {
    //    None
    //};

    let payload = construct_service_provider_announce_sign_payload(nonce, address, coin, service);
    let wrapper = DataWrapper::new(payload.to_base58_string().unwrap());
    println!("{}", args.output.format(&wrapper))
}
