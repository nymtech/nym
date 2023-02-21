// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use mixnet_contract_common::MixId;
use validator_client::nyxd::traits::{MixnetQueryClient, MixnetSigningClient};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub mix_id: Option<MixId>,

    #[clap(long)]
    pub identity_key: Option<String>,
}

pub async fn undelegate_from_mixnode(args: Args, client: SigningClient) {
    info!("removing stake from mix-node");

    let mix_id = match args.mix_id {
        Some(mix_id) => mix_id,
        None => {
            let identity_key = args
                .identity_key
                .expect("either mix_id or mix_identity has to be specified");
            let node_details = client
                .get_mixnode_details_by_identity(identity_key)
                .await
                .expect("contract query failed")
                .expect("mixnode with the specified identity doesnt exist");
            node_details.mix_id()
        }
    };

    let res = client
        .undelegate_from_mixnode(mix_id, None)
        .await
        .expect("failed to remove stake from mixnode!");

    info!("removing stake from mixnode: {:?}", res)
}
