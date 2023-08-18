// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_mixnet_contract_common::GatewayConfigUpdate;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, MixnetSigningClient};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub host: Option<String>,

    #[clap(long)]
    pub mix_port: Option<u16>,

    #[clap(long)]
    pub clients_port: Option<u16>,

    #[clap(long)]
    pub location: Option<String>,

    #[clap(long)]
    pub version: Option<String>,
}

pub async fn update_config(args: Args, client: SigningClient) {
    info!("Update gateway config!");

    let current_details = match client
        .get_owned_gateway(&client.address())
        .await
        .expect("failed to query the chain for gateway details")
        .gateway
    {
        Some(details) => details,
        None => {
            log::warn!("this operator does not own a gateway to update");
            return;
        }
    };

    let update = GatewayConfigUpdate {
        host: args.host.unwrap_or(current_details.gateway.host),
        mix_port: args.mix_port.unwrap_or(current_details.gateway.mix_port),
        clients_port: args
            .clients_port
            .unwrap_or(current_details.gateway.clients_port),
        location: args.location.unwrap_or(current_details.gateway.location),
        version: args.version.unwrap_or(current_details.gateway.version),
    };

    let res = client
        .update_gateway_config(update, None)
        .await
        .expect("updating gateway config");

    info!("gateway config updated: {:?}", res)
}
