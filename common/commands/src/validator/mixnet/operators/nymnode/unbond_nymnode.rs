// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::info;
use nym_validator_client::nyxd::contract_traits::MixnetSigningClient;

use crate::context::SigningClient;

#[derive(Debug, Parser)]
pub struct Args {}

pub async fn unbond_nymnode(_args: Args, client: SigningClient) {
    info!("Starting Nym Node unbonding!");

    let res = client
        .unbond_nymnode(None)
        .await
        .expect("failed to unbond Nym Node!");

    info!("Unbonding result: {:?}", res)
}
