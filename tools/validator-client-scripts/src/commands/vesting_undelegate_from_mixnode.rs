// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Client;
use clap::Parser;
use log::info;
use validator_client::nymd::VestingSigningClient;

#[derive(Debug, Parser)]
pub(crate) struct Args {
    #[clap(long)]
    pub identity_key: String,

    #[clap(long)]
    pub gas: Option<u64>,
}

pub(crate) async fn vesting_undelegate_from_mixnode(client: Client, args: Args) {
    info!("removing stake from vesting mix-node");

    let res = client
        .vesting_undelegate_from_mixnode(&*args.identity_key, None)
        .await
        .expect("failed to remove stake from vesting account on mixnode!");

    info!("removing stake from vesting mixnode: {:?}", res)
}
