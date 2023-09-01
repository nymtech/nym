// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_validator_client::nyxd::contract_traits::MixnetSigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub gas: Option<u64>,
}

pub async fn vesting_claim_operator_reward(client: SigningClient) {
    info!("Claim vesting operator reward");

    let res = client
        .withdraw_operator_reward_on_behalf(client.address().clone(), None)
        .await
        .expect("failed to claim vesting operator reward");

    info!("Claiming vesting operator reward: {:?}", res)
}
