// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Client;
use clap::Parser;
use log::info;

#[derive(Debug, Parser)]
pub(crate) struct Args {
    #[clap(long)]
    pub gas: Option<u64>,

    #[clap(long)]
    pub identity: String,
}

pub(crate) async fn claim_delegator_reward(client: Client, args: Args) {
    info!("Claim delegator reward");

    let res = client
        .execute_claim_delegator_reward(args.identity, None)
        .await
        .expect("failed to claim delegator-reward");

    info!("Claiming delegator reward: {:?}", res)
}
