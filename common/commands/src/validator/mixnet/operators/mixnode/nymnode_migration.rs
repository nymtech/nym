// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_validator_client::nyxd::contract_traits::MixnetSigningClient;

#[derive(Debug, Parser)]
pub struct Args {}

pub async fn migrate_to_nymnode(_args: Args, client: SigningClient) {
    let res = client
        .migrate_legacy_mixnode(None)
        .await
        .expect("failed to migrate mixnode!");

    info!("migration result: {:?}", res)
}
