// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_validator_client::nyxd::contract_traits::MixnetSigningClient;

#[derive(Debug, Parser)]
pub struct Args {}

pub async fn vesting_unbond_gateway(client: SigningClient) {
    info!("Starting vesting gateway unbonding!");

    let res = client
        .unbond_gateway(None)
        .await
        .expect("failed to unbond vesting gateway!");

    info!("Unbonding vesting result: {:?}", res)
}
