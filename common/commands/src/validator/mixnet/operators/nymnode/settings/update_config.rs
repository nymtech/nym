// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_mixnet_contract_common::nym_node::NodeConfigUpdate;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, MixnetSigningClient};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub host: Option<String>,

    // ideally this would have been `Option<Option<u16>>`, but not sure if clap would have recognised it
    #[clap(long)]
    pub custom_http_port: Option<u16>,

    // equivalent to setting `custom_http_port` to `None`
    #[clap(long)]
    pub restore_default_http_port: bool,
}

pub async fn update_config(args: Args, client: SigningClient) {
    info!("Update nym node config!");

    if client
        .get_owned_nymnode(&client.address())
        .await
        .expect("failed to query the chain for nym node details")
        .details
        .is_none()
    {
        log::warn!("this operator does not own a nym node to update");
        return;
    }

    let update = NodeConfigUpdate {
        host: args.host,
        custom_http_port: args.custom_http_port,
        restore_default_http_port: args.restore_default_http_port,
    };

    let res = client
        .update_nymnode_config(update, None)
        .await
        .expect("updating nym node config");

    info!("nym node config updated: {:?}", res)
}
