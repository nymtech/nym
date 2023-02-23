// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::OverrideConfig;
use crate::commands::{override_config, version_check};
use crate::config::Config;
use crate::node::MixNode;
use crate::OutputFormat;
use clap::Args;
use nym_config::NymConfig;
use std::net::IpAddr;
use validator_client::nyxd;

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

    /// The host that will be reported to the directory server
    #[clap(long)]
    announce_host: Option<String>,

    /// Comma separated list of nym-api endpoints of the validators
    // the alias here is included for backwards compatibility (1.1.4 and before)
    #[clap(long, alias = "validators", value_delimiter = ',')]
    nym_apis: Option<Vec<url::Url>>,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            id: run_config.id,
            host: run_config.host,
            wallet_address: run_config.wallet_address,
            mix_port: run_config.mix_port,
            verloc_port: run_config.verloc_port,
            http_api_port: run_config.http_api_port,
            announce_host: run_config.announce_host,
            nym_apis: run_config.nym_apis,
        }
    }
}

fn show_binding_warning(address: &str) {
    println!("\n##### NOTE #####");
    println!(
        "\nYou are trying to bind to {address} - you might not be accessible to other nodes\n\
         You can ignore this note if you're running setup on a local network \n\
         or have set a custom 'announce-host'"
    );
    println!("\n\n");
}

fn special_addresses() -> Vec<&'static str> {
    vec!["localhost", "127.0.0.1", "0.0.0.0", "::1", "[::1]"]
}

pub(crate) async fn execute(args: &Run, output: OutputFormat) {
    eprintln!("Starting mixnode {}...", args.id);

    let mut config = match Config::load_from_file(&args.id) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!(
                "Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})",
                args.id,
                err
            );
            return;
        }
    };

    config = override_config(config, OverrideConfig::from(args.clone()));

    if !version_check(&config) {
        error!("failed the local version check");
        return;
    }

    if special_addresses().contains(&&*config.get_listening_address().to_string()) {
        show_binding_warning(&config.get_listening_address().to_string());
    }

    let mut mixnode = MixNode::new(config);

    eprintln!(
        "\nTo bond your mixnode you will need to install the Nym wallet, go to https://nymtech.net/get-involved and select the Download button.\n\
         Select the correct version and install it to your machine. You will need to provide the following: \n ");
    mixnode.print_node_details(output);

    mixnode.run().await
}
