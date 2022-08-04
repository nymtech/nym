// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Client;
use clap::Parser;
use log::info;

#[derive(Debug, Parser)]
pub(crate) struct Args {
    #[clap(long)]
    pub gas: Option<u64>,
}

pub(crate) async fn vesting_claim_operator_reward(client: Client) {
    info!("Claim vesting operator reward");

    let res = client
        .execute_vesting_claim_operator_reward(None)
        .await
        .expect("failed to claim vesting operator reward");

    info!("Claiming vesting operator reward: {:?}", res)
}
