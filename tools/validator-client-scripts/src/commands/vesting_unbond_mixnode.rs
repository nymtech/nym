// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Client;
use clap::Parser;
use log::info;
use validator_client::nymd::VestingSigningClient;

#[derive(Debug, Parser)]
pub(crate) struct Args {
    #[clap(long)]
    pub gas: Option<u64>,
}

pub(crate) async fn vesting_unbond_mixnode(client: Client) {
    info!("Starting vesting mixnode unbonding!");

    let res = client
        .vesting_unbond_mixnode(None)
        .await
        .expect("failed to unbond vesting mixnode!");

    info!("Unbonding vesting result: {:?}", res)
}
