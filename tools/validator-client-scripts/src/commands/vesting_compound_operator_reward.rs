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

pub(crate) async fn vesting_compound_operator_reward(client: Client, _args: Args) {
    info!("compounding vesting operator reward");

    let res = client
        .execute_vesting_compound_operator_reward(None)
        .await
        .expect("failed to compound operator-reward");

    info!("Claiming compound operator reward: {:?}", res)
}
