// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_mixnet_contract_common::MixNodeConfigUpdate;
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use nym_validator_client::nyxd::contract_traits::VestingSigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub host: Option<String>,

    #[clap(long)]
    pub mix_port: Option<u16>,

    #[clap(long)]
    pub verloc_port: Option<u16>,

    #[clap(long)]
    pub http_api_port: Option<u16>,

    #[clap(long)]
    pub version: Option<String>,
}

pub async fn vesting_update_config(client: SigningClient, args: Args) {
    info!("Update vesting mix node config!");

    let current_details = match client
        .get_owned_mixnode(&client.address())
        .await
        .expect("failed to query the chain for mixnode details")
        .mixnode_details
    {
        Some(details) => details,
        None => {
            log::warn!("this operator does not own a mixnode to update");
            return;
        }
    };

    let update = MixNodeConfigUpdate {
        host: args
            .host
            .unwrap_or(current_details.bond_information.mix_node.host),
        mix_port: args
            .mix_port
            .unwrap_or(current_details.bond_information.mix_node.mix_port),
        verloc_port: args
            .verloc_port
            .unwrap_or(current_details.bond_information.mix_node.verloc_port),
        http_api_port: args
            .http_api_port
            .unwrap_or(current_details.bond_information.mix_node.http_api_port),
        version: args
            .version
            .unwrap_or(current_details.bond_information.mix_node.version),
    };

    let res = client
        .vesting_update_mixnode_config(update, None)
        .await
        .expect("updating vesting mix-node config");

    info!("mixnode config updated: {:?}", res)
}
