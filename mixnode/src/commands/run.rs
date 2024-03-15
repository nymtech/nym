// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::OverrideConfig;
use crate::commands::{override_config, try_load_current_config, version_check};
use crate::node::MixNode;
use anyhow::bail;
use clap::Args;
use log::error;
use nym_bin_common::output_format::OutputFormat;
use nym_config::helpers::SPECIAL_ADDRESSES;
use nym_validator_client::nyxd;
use std::net::IpAddr;
#[derive(Args, Clone)]
pub(crate) struct Run {
    /// Id of the nym-mixnode we want to run
    #[clap(long)]
    id: String,

    /// The custom host on which the mixnode will be running
    #[clap(long)]
    host: Option<IpAddr>,

    /// The wallet address you will use to bond this mixnode, e.g. nymt1z9egw0knv47nmur0p8vk4rcx59h9gg4zuxrrr9
    #[clap(long)]
    wallet_address: Option<nyxd::AccountId>,

    /// The port on which the mixnode will be listening for mix packets
    #[clap(long)]
    mix_port: Option<u16>,

    /// The port on which the mixnode will be listening for verloc packets
    #[clap(long)]
    verloc_port: Option<u16>,

    /// The port on which the mixnode will be listening for http requests
    #[clap(long)]
    http_api_port: Option<u16>,

    /// Comma separated list of nym-api endpoints of the validators
    // the alias here is included for backwards compatibility (1.1.4 and before)
    #[clap(long, alias = "validators", value_delimiter = ',')]
    nym_apis: Option<Vec<url::Url>>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,

    #[clap(long)]
    metrics_key: Option<String>,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            id: run_config.id,
            host: run_config.host,
            mix_port: run_config.mix_port,
            verloc_port: run_config.verloc_port,
            http_api_port: run_config.http_api_port,
            nym_apis: run_config.nym_apis,
            metrics_key: run_config.metrics_key,
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
