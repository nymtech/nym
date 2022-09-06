// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub identity_key: String,
}

pub async fn claim_delegator_reward(args: Args, client: SigningClient) {
    info!("Claim delegator reward");

    let res = client
        .execute_claim_delegator_reward(args.identity_key, None)
        .await
        .expect("failed to claim delegator-reward");

    info!("Claiming delegator reward: {:?}", res)
}
