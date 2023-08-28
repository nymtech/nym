// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    context::SigningClient,
    utils::{account_id_to_cw_addr, DataWrapper},
};

use clap::Parser;
use cosmwasm_std::Coin;

use nym_bin_common::output_format::OutputFormat;
use nym_name_service_common::{
    signing_types::construct_name_register_sign_payload, Address, NymName,
};
use nym_sphinx::addressing::clients::Recipient;
use nym_validator_client::nyxd::{contract_traits::NameServiceQueryClient, error::NyxdError};

#[derive(Debug, Parser)]
pub struct Args {
    // The name to register.
    #[arg(long)]
    pub name: NymName,

    /// The nym address the name should point to.
    #[arg(long)]
    pub nym_address: Recipient,

    #[arg(long)]
    pub amount: u128,

    #[arg(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub async fn create_payload(args: Args, client: SigningClient) -> Result<(), NyxdError> {
    let address = Address::new(&args.nym_address.to_string()).expect("invalid address");
    let identity_key = address.client_id().to_string();
    let name = nym_name_service_common::NameDetails {
        name: args.name,
        address,
        identity_key,
    };

    let denom = client.current_chain_details().mix_denom.base.as_str();
    let deposit = Coin::new(args.amount, denom);

    let nonce = match client.get_name_signing_nonce(&client.address()).await {
        Ok(nonce) => nonce,
        Err(err) => {
            eprint!(
                "failed to query for the signing nonce of {}: {err}",
                client.address()
            );
            return Err(err);
        }
    };

    let address = account_id_to_cw_addr(&client.address());
    let payload = construct_name_register_sign_payload(nonce, address, deposit, name);
    let wrapper = DataWrapper::new(payload.to_base58_string().unwrap());
    println!("{}", args.output.format(&wrapper));

    Ok(())
}
