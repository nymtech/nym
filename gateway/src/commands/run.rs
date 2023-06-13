// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::{ensure_config_version_compatibility, OverrideConfig};
use crate::support::config::build_config;
use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use nym_config::helpers::SPECIAL_ADDRESSES;
use std::error::Error;
use std::net::IpAddr;
use std::path::PathBuf;

#[derive(Args, Clone)]
pub struct Run {
    /// Id of the gateway we want to run
    #[clap(long)]
    id: String,

    /// The custom host on which the gateway will be running for receiving sphinx packets
    #[clap(long)]
    host: Option<IpAddr>,

    /// The port on which the gateway will be listening for sphinx packets
    #[clap(long)]
    mix_port: Option<u16>,

    /// The port on which the gateway will be listening for clients gateway-requests
    #[clap(long)]
    clients_port: Option<u16>,

    /// Path to sqlite database containing all gateway persistent data
    #[clap(long)]
    datastore: Option<PathBuf>,

    /// Comma separated list of endpoints of nym APIs
    #[clap(long, alias = "validator_apis", value_delimiter = ',')]
    // the alias here is included for backwards compatibility (1.1.4 and before)
    nym_apis: Option<Vec<url::Url>>,

    /// Comma separated list of endpoints of the validator
    #[clap(
        long,
        alias = "validators",
        alias = "nyxd_validators",
        value_delimiter = ',',
        hide = true
    )]
    // the alias here is included for backwards compatibility (1.1.4 and before)
    nyxd_urls: Option<Vec<url::Url>>,

    /// Cosmos wallet mnemonic
    #[clap(long)]
    mnemonic: Option<bip39::Mnemonic>,

    /// Set this gateway to work only with coconut credentials; that would disallow clients to
    /// bypass bandwidth credential requirement
    #[clap(long, hide = true)]
    only_coconut_credentials: Option<bool>,

    /// Enable/disable gateway anonymized statistics that get sent to a statistics aggregator server
    #[clap(long)]
    enabled_statistics: Option<bool>,

    /// URL where a statistics aggregator is running. The default value is a Nym aggregator server
    #[clap(long)]
    statistics_service_url: Option<url::Url>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            host: run_config.host,
            mix_port: run_config.mix_port,
            clients_port: run_config.clients_port,
            datastore: run_config.datastore,
            nym_apis: run_config.nym_apis,
            mnemonic: run_config.mnemonic,

            enabled_statistics: run_config.enabled_statistics,
            statistics_service_url: run_config.statistics_service_url,
            nyxd_urls: run_config.nyxd_urls,
            only_coconut_credentials: run_config.only_coconut_credentials,
        }
    }
}

fn show_binding_warning(address: &str) {
    eprintln!("\n##### NOTE #####");
    eprintln!(
        "\nYou are trying to bind to {address} - you might not be accessible to other nodes\n\
         You can ignore this warning if you're running setup on a local network \n\
         or have used different host when bonding your node"
    );
    eprintln!("\n\n");
}

pub async fn execute(args: Run) -> Result<(), Box<dyn Error + Send + Sync>> {
    let id = args.id.clone();
    eprintln!("Starting gateway {id}...");

    let output = args.output;
    let config = build_config(id, args)?;
    ensure_config_version_compatibility(&config)?;

    if SPECIAL_ADDRESSES.contains(&config.gateway.listening_address) {
        show_binding_warning(&config.gateway.listening_address.to_string());
    }

    let mut gateway = crate::node::create_gateway(config).await;
    eprintln!(
        "\nTo bond your gateway you will need to install the Nym wallet, go to https://nymtech.net/get-involved and select the Download button.\n\
         Select the correct version and install it to your machine. You will need to provide the following: \n ");
    gateway.print_node_details(output);

    gateway.run().await
}
