// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::Config,
    node::{storage::PersistentStorage, Gateway},
};
use clap::Args;
use config::NymConfig;
use log::error;

#[derive(Args, Clone)]
pub struct NodeDetails {
    /// The id of the gateway you want to show details for
    #[clap(long)]
    id: String,
}

pub async fn execute(args: &NodeDetails) {
    let config = match Config::load_from_file(Some(&args.id)) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!(
                "Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})",
                args.id,
                err,
            );
            return;
        }
    };

    Gateway::<PersistentStorage>::new(config)
        .await
        .print_node_details();
}
