// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Client;
use clap::Parser;
use log::info;

#[derive(Debug, Parser)]
pub(crate) struct Args {
    #[clap(long)]
    pub identity_key: String,

    #[clap(long)]
    pub gas: Option<u64>,
}

pub(crate) async fn undelegate_from_mixnode(client: Client, args: Args) {
    info!("removing stake from mix-node");

    let res = client
        .remove_mixnode_delegation(&*args.identity_key, None)
        .await
        .expect("failed to remove stake from mixnode!");

    info!("removing stake from mixnode: {:?}", res)
}
