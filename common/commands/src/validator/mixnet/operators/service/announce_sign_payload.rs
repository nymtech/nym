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
use nym_validator_client::nyxd::contract_traits::SpDirectoryQueryClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub nym_address: Recipient,

    #[clap(long)]
    pub amount: u128,

    #[clap(long)]
    pub identity_key: String,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub async fn create_payload(args: Args, client: SigningClient) {
    let service = nym_service_provider_directory_common::ServiceDetails {
        nym_address: NymAddress::new(&args.nym_address.to_string()),
        service_type: NetworkRequester,
        identity_key: args.identity_key,
    };

    let denom = client.current_chain_details().mix_denom.base.as_str();
    let deposit = Coin::new(args.amount, denom);

    let nonce = match client.get_service_signing_nonce(&client.address()).await {
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
        construct_service_provider_announce_sign_payload(nonce, address, deposit, service);
    let wrapper = DataWrapper::new(payload.to_base58_string().unwrap());
    println!("{}", args.output.format(&wrapper))
}
