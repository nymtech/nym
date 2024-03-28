// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_validator_client::nyxd::contract_traits::MixnetSigningClient;

#[derive(Debug, Parser)]
pub struct Args {}

pub async fn claim_operator_reward(_args: Args, client: SigningClient) {
    info!("Claim operator reward");

    let res = client
        .withdraw_operator_reward(None)
        .await
        .expect("failed to claim operator reward");

    info!("Claiming operator reward: {:?}", res)
}
