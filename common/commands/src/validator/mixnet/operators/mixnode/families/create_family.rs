// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_validator_client::nyxd::contract_traits::MixnetSigningClient;
use nym_validator_client::nyxd::contract_traits::VestingSigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    /// Label that is going to be used for creating the family
    #[arg(long)]
    pub family_label: String,

    /// Indicates whether the family is going to get created via a vesting account
    #[arg(long)]
    pub with_vesting_account: bool,
}

pub async fn create_family(args: Args, client: SigningClient) {
    info!("Create family");

    let res = if args.with_vesting_account {
        client
            .vesting_create_family(args.family_label, None)
            .await
            .expect("failed to create family with vesting account")
    } else {
        client
            .create_family(args.family_label, None)
            .await
            .expect("failed to create family")
    };

    info!("Family creation result: {:?}", res);
}
