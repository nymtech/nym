// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::QueryClient;
use crate::utils::account_id_to_cw_addr;
use clap::Parser;
use cosmrs::AccountId;
use log::info;
use nym_mixnet_contract_common::construct_family_creation_sign_payload;
use validator_client::nyxd::traits::MixnetQueryClient;

#[derive(Debug, Parser)]
pub struct Args {
    /// Account address which will be used for creating the family
    #[arg(long)]
    pub address: AccountId,

    /// Label that is going to be used for creating the family
    #[arg(long)]
    pub family_label: String,

    /// Indicates whether the family is going to get created via a vesting account
    #[arg(long)]
    pub with_vesting_account: bool,
}

pub async fn create_family_creation_sign_payload(args: Args, client: QueryClient) {
    info!("Create family creation sign payload");

    let nonce = match client.get_signing_nonce(&args.address).await {
        Ok(nonce) => nonce,
        Err(err) => {
            eprint!(
                "failed to query for the signing nonce of {}: {err}",
                args.address
            );
            return;
        }
    };

    let address = account_id_to_cw_addr(&args.address);
    let proxy = if args.with_vesting_account {
        Some(account_id_to_cw_addr(client.vesting_contract_address()))
    } else {
        None
    };

    let payload = construct_family_creation_sign_payload(nonce, address, proxy, args.family_label);
    println!("{}", payload.to_base58_string().unwrap())
}
