// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::support::config::build_config;
use crate::{
    commands::{ensure_config_version_compatibility, OverrideConfig},
    OutputFormat,
};
use clap::Args;
use std::error::Error;
use std::net::IpAddr;
use std::path::PathBuf;
use validator_client::nyxd;

#[derive(Args, Clone)]
pub struct Run {
    /// Id of the gateway we want to run
    #[clap(long)]
    id: String,

    /// The custom host on which the gateway will be running for receiving sphinx packets
    #[clap(long)]
    host: Option<IpAddr>,

    /// The wallet address you will use to bond this gateway, e.g. nymt1z9egw0knv47nmur0p8vk4rcx59h9gg4zuxrrr9
    #[clap(long)]
    wallet_address: Option<nyxd::AccountId>,

    /// The port on which the gateway will be listening for sphinx packets
    #[clap(long)]
    mix_port: Option<u16>,

    /// The port on which the gateway will be listening for clients gateway-requests
    #[clap(long)]
    clients_port: Option<u16>,

    /// The host that will be reported to the directory server
    #[clap(long)]
    // TODO: could this be changed to `Option<url::Url>`?
    announce_host: Option<String>,

    /// Path to sqlite database containing all gateway persistent data
    #[clap(long)]
    datastore: Option<PathBuf>,

    /// Comma separated list of endpoints of nym APIs
    #[clap(long, alias = "validator_apis", value_delimiter = ',')]
    // the alias here is included for backwards compatibility (1.1.4 and before)
    nym_apis: Option<Vec<url::Url>>,

    /// Comma separated list of endpoints of the validator
    #[cfg(feature = "coconut")]
    #[clap(
        long,
        alias = "validators",
        alias = "nymd_validators",
        value_delimiter = ','
    )]
    // the alias here is included for backwards compatibility (1.1.4 and before)
    nyxd_urls: Option<Vec<url::Url>>,

    /// Cosmos wallet mnemonic
    #[clap(long)]
    mnemonic: Option<bip39::Mnemonic>,

    /// Set this gateway to work only with coconut credentials; that would disallow clients to
    /// bypass bandwidth credential requirement
    #[cfg(feature = "coconut")]
    #[clap(long)]
    only_coconut_credentials: Option<bool>,

    /// Enable/disable gateway anonymized statistics that get sent to a statistics aggregator server
    #[clap(long)]
    enabled_statistics: Option<bool>,

    /// URL where a statistics aggregator is running. The default value is a Nym aggregator server
    #[clap(long)]
    statistics_service_url: Option<url::Url>,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            host: run_config.host,
            wallet_address: run_config.wallet_address,
            mix_port: run_config.mix_port,
            clients_port: run_config.clients_port,
            datastore: run_config.datastore,
            announce_host: run_config.announce_host,
            nym_apis: run_config.nym_apis,
            mnemonic: run_config.mnemonic,

            enabled_statistics: run_config.enabled_statistics,
            statistics_service_url: run_config.statistics_service_url,

            #[cfg(feature = "coconut")]
            nyxd_urls: run_config.nyxd_urls,
            #[cfg(feature = "coconut")]
            only_coconut_credentials: run_config.only_coconut_credentials,
        }
    }
}

fn show_binding_warning(address: String) {
    eprintln!("\n##### NOTE #####");
    eprintln!(
        "\nYou are trying to bind to {} - you might not be accessible to other nodes\n\
         You can ignore this warning if you're running setup on a local network \n\
         or have set a custom 'announce-host'",
        address
    );
    eprintln!("\n\n");
}

fn special_addresses() -> Vec<&'static str> {
    vec!["localhost", "127.0.0.1", "0.0.0.0", "::1", "[::1]"]
}

pub async fn execute(args: Run, output: OutputFormat) -> Result<(), Box<dyn Error + Send + Sync>> {
    let id = args.id.clone();
    eprintln!("Starting gateway {id}...");

    let config = build_config(id, args)?;
    ensure_config_version_compatibility(&config)?;

    if special_addresses().contains(&&*config.get_listening_address().to_string()) {
        show_binding_warning(config.get_listening_address().to_string());
    }

    let mut gateway = crate::node::create_gateway(config).await;
    eprintln!(
        "\nTo bond your gateway you will need to install the Nym wallet, go to https://nymtech.net/get-involved and select the Download button.\n\
         Select the correct version and install it to your machine. You will need to provide the following: \n ");
    gateway.print_node_details(output)?;

    gateway.run().await
}
