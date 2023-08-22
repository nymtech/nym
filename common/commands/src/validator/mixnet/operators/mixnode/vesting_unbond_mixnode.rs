// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_validator_client::nyxd::contract_traits::VestingSigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub gas: Option<u64>,
}

pub async fn vesting_unbond_mixnode(client: SigningClient) {
    info!("Starting vesting mixnode unbonding!");

    let res = client
        .vesting_unbond_mixnode(None)
        .await
        .expect("failed to unbond vesting mixnode!");

    info!("Unbonding vesting result: {:?}", res)
}
