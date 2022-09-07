// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub identity: String,
}

pub async fn vesting_claim_delegator_reward(args: Args, client: SigningClient) {
    info!("Claim vesting delegator reward");

    let res = client
        .execute_vesting_claim_delegator_reward(args.identity, None)
        .await
        .expect("failed to claim vesting delegator-reward");

    info!("Claiming vesting delegator reward: {:?}", res)
}
