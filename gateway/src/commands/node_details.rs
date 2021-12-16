// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::*;
use crate::config::Config;
use crate::node::Gateway;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use log::error;

pub fn command_args<'a, 'b>() -> App<'a, 'b> {
    App::new("node-details")
        .about("Show details of this gateway")
        .arg(
            Arg::with_name(ID_ARG_NAME)
                .long(ID_ARG_NAME)
                .help("The id of the gateway you want to show details for")
                .takes_value(true)
                .required(true),
        )
}

pub async fn execute(matches: ArgMatches<'static>) {
    let id = matches.value_of(ID_ARG_NAME).unwrap();

    let config = match Config::load_from_file(Some(id)) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})", id, err);
            return;
        }
    };

    Gateway::new(config).await.print_node_details()
}
