// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_validator_client::nyxd::contract_traits::MixnetSigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    /// Label that is going to be used for creating the family
    #[arg(long)]
    pub family_label: String,
}

pub async fn create_family(args: Args, client: SigningClient) {
    info!("Create family");

    let res = client
        .create_family(args.family_label, None)
        .await
        .expect("failed to create family");

    info!("Family creation result: {:?}", res);
}
