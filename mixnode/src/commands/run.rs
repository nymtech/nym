// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::OverrideConfig;
use super::DEFAULT_MIXNODE_ID;
use crate::commands::{override_config, try_load_current_config, version_check};
use crate::env::vars::*;
use crate::node::MixNode;
use anyhow::bail;
use clap::Args;
use log::error;
use nym_bin_common::output_format::OutputFormat;
use nym_config::helpers::SPECIAL_ADDRESSES;
use std::net::IpAddr;

#[derive(Args, Clone)]
pub(crate) struct Run {
    /// Id of the nym-mixnode we want to run
    #[clap(long, default_value = DEFAULT_MIXNODE_ID, env = MIXNODE_ID_ARG)]
    id: String,

    /// The custom host on which the mixnode will be running
    #[clap(long, alias = "host", env = MIXNODE_LISTENING_ADDRESS_ARG)]
    listening_address: Option<IpAddr>,

    /// The port on which the mixnode will be listening for mix packets
    #[clap(long, env = MIXNODE_MIX_PORT_ARG)]
    mix_port: Option<u16>,

    /// The port on which the mixnode will be listening for verloc packets
    #[clap(long, env = MIXNODE_VERLOC_PORT_ARG)]
    verloc_port: Option<u16>,

    /// The port on which the mixnode will be listening for http requests
    #[clap(long, env = MIXNODE_HTTP_API_PORT_ARG)]
    http_api_port: Option<u16>,

    /// Comma separated list of nym-api endpoints of the validators
    // the alias here is included for backwards compatibility (1.1.4 and before)
    #[clap(long, alias = "validators", value_delimiter = ',', env = MIXNODE_NYM_APIS_ARG)]
    nym_apis: Option<Vec<url::Url>>,

    #[clap(short, long, default_value_t = OutputFormat::default(), env = MIXNODE_OUTPUT_ARG)]
    output: OutputFormat,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            id: run_config.id,
            listening_address: run_config.listening_address,
            mix_port: run_config.mix_port,
            verloc_port: run_config.verloc_port,
            http_api_port: run_config.http_api_port,
            nym_apis: run_config.nym_apis,
        }
    }
}

fn show_binding_warning(address: &str) {
    eprintln!("\n##### NOTE #####");
    eprintln!(
        "\nYou are trying to bind to {address} - you might not be accessible to other nodes\n\
         You can ignore this note if you're running setup on a local network \n\
         or have used different host when bonding your node"
    );
    eprintln!("\n\n");
}

pub(crate) async fn execute(args: &Run) -> anyhow::Result<()> {
    eprintln!("Starting mixnode {}...", args.id);

    let mut config = try_load_current_config(&args.id)?;
    config = override_config(config, OverrideConfig::from(args.clone()));

    if !version_check(&config) {
        error!("failed the local version check");
        bail!("failed the local version check")
    }

    if SPECIAL_ADDRESSES.contains(&config.mixnode.listening_address) {
        show_binding_warning(&config.mixnode.listening_address.to_string());
    }

    let mut mixnode = MixNode::new(config)?;

    eprintln!(
        "\nTo bond your mixnode you will need to install the Nym wallet, go to https://nymtech.net/get-involved and select the Download button.\n\
         Select the correct version and install it to your machine. You will need to provide the following: \n ");
    mixnode.print_node_details(args.output);

    mixnode.run().await?;
    Ok(())
}
