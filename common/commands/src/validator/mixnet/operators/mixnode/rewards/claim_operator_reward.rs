// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use validator_client::nyxd::traits::MixnetSigningClient;

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
