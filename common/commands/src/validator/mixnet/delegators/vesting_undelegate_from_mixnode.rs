// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::info;

use validator_client::nymd::VestingSigningClient;

use crate::context::SigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub identity_key: String,
}

pub async fn vesting_undelegate_from_mixnode(args: Args, client: SigningClient) {
    info!("removing stake from vesting mix-node");

    let res = client
        .vesting_undelegate_from_mixnode(&*args.identity_key, None)
        .await
        .expect("failed to remove stake from vesting account on mixnode!");

    info!("removing stake from vesting mixnode: {:?}", res)
}
