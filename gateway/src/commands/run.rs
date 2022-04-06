// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    commands::{override_config, version_check, OverrideConfig},
    config::Config,
};
use clap::Args;
use config::NymConfig;
use log::*;

#[derive(Args, Clone)]
pub struct Run {
    /// Id of the gateway we want to run
    #[clap(long)]
    id: String,

    /// The custom host on which the gateway will be running for receiving sphinx packets
    #[clap(long)]
    host: Option<String>,

    /// The wallet address you will use to bond this gateway, e.g. nymt1z9egw0knv47nmur0p8vk4rcx59h9gg4zuxrrr9
    #[clap(long)]
    wallet_address: Option<String>,

    /// The port on which the gateway will be listening for sphinx packets
    #[clap(long)]
    mix_port: Option<u16>,

    /// The port on which the gateway will be listening for clients gateway-requests
    #[clap(long)]
    clients_port: Option<u16>,

    /// The host that will be reported to the directory server
    #[clap(long)]
    announce_host: Option<String>,

    /// Path to sqlite database containing all gateway persistent data
    #[clap(long)]
    datastore: Option<String>,

    /// Comma separated list of endpoints of the validators APIs
    #[clap(long)]
    validator_apis: Option<String>,

    /// Cosmos wallet mnemonic
    #[clap(long)]
    mnemonic: Option<String>,

    /// Set this gateway to work in a testnet mode that would allow clients to bypass bandwidth credential requirement
    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    #[clap(long)]
    testnet_mode: bool,

    /// URL of an Ethereum full node that we want to use for getting bandwidth tokens from ERC20 tokens
    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    #[clap(long)]
    eth_endpoint: Option<String>,

    /// Comma separated list of endpoints of the validator
    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    #[clap(long)]
    validators: Option<String>,
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
            validator_apis: run_config.validator_apis,
            mnemonic: run_config.mnemonic,

            #[cfg(all(feature = "eth", not(feature = "coconut")))]
            testnet_mode: run_config.testnet_mode,

            #[cfg(all(feature = "eth", not(feature = "coconut")))]
            eth_endpoint: run_config.eth_endpoint,

            #[cfg(all(feature = "eth", not(feature = "coconut")))]
            validators: run_config.validators,
        }
    }
}

fn show_binding_warning(address: String) {
    println!("\n##### NOTE #####");
    println!(
        "\nYou are trying to bind to {} - you might not be accessible to other nodes\n\
         You can ignore this warning if you're running setup on a local network \n\
         or have set a custom 'announce-host'",
        address
    );
    println!("\n\n");
}

fn special_addresses() -> Vec<&'static str> {
    vec!["localhost", "127.0.0.1", "0.0.0.0", "::1", "[::1]"]
}

pub async fn execute(args: &Run) {
    println!("Starting gateway {}...", args.id);

    let mut config = match Config::load_from_file(Some(&args.id)) {
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

    config = override_config(config, OverrideConfig::from(args.clone()));

    if !version_check(&config) {
        error!("failed the local version check");
        return;
    }

    if special_addresses().contains(&&*config.get_listening_address().to_string()) {
        show_binding_warning(config.get_listening_address().to_string());
    }

    let mut gateway = crate::node::create_gateway(config).await;
    println!(
        "\nTo bond your gateway you will need to install the Nym wallet, go to https://nymtech.net/get-involved and select the Download button.\n\
         Select the correct version and install it to your machine. You will need to provide the following: \n ");
    gateway.print_node_details();

    gateway.run().await;
}
