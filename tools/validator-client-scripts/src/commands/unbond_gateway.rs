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

pub(crate) async fn unbond_gateway(client: Client) {
    info!("Starting gateway unbonding!");

    let res = client
        .unbond_gateway(None)
        .await
        .expect("failed to unbond gateway!");

    info!("Unbonding result: {:?}", res)
}
