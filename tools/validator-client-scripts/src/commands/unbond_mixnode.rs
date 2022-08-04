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

pub(crate) async fn unbond_mixnode(client: Client) {
    info!("Starting mixnode unbonding!");

    let res = client
        .unbond_mixnode(None)
        .await
        .expect("failed to unbond mixnode!");

    info!("Unbonding result: {:?}", res)
}
