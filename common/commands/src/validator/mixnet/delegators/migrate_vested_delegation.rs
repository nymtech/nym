// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use clap::Parser;
use log::info;
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, MixnetSigningClient};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub mix_id: Option<NodeId>,

    #[clap(long)]
    pub identity_key: Option<String>,
}

pub async fn migrate_vested_delegation(args: Args, client: SigningClient) {
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
                .mixnode_details
                .expect("mixnode with the specified identity doesnt exist");
            node_details.mix_id()
        }
    };

    let res = client
        .migrate_vested_delegation(mix_id, None)
        .await
        .expect("failed to migrate delegation!");

    info!("migration result: {:?}", res)
}
