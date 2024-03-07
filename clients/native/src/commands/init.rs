// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::{
    default_config_directory, default_config_filepath, default_data_directory,
};
use crate::commands::try_upgrade_config;
use crate::{
    client::config::Config,
    commands::{override_config, OverrideConfig},
    error::ClientError,
};
use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::cli_helpers::client_init::{
    initialise_client, CommonClientInitArgs, InitResultsWithConfig, InitialisableClient,
};
use serde::Serialize;
use std::fmt::Display;
use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;

struct NativeClientInit;

impl InitialisableClient for NativeClientInit {
    const NAME: &'static str = "native";
    type Error = ClientError;
    type InitArgs = Init;
    type Config = Config;

    async fn try_upgrade_outdated_config(id: &str) -> Result<(), Self::Error> {
        try_upgrade_config(id).await
    }

    fn initialise_storage_paths(id: &str) -> Result<(), Self::Error> {
        fs::create_dir_all(default_data_directory(id))?;
        fs::create_dir_all(default_config_directory(id))?;
        Ok(())
    }

    fn default_config_path(id: &str) -> PathBuf {
        default_config_filepath(id)
    }

    fn construct_config(init_args: &Self::InitArgs) -> Self::Config {
        override_config(
            Config::new(&init_args.common_args.id),
            OverrideConfig::from(init_args.clone()),
        )
    }
}

#[derive(Args, Clone, Debug)]
pub(crate) struct Init {
    #[command(flatten)]
    common_args: CommonClientInitArgs,

    /// Whether to not start the websocket
    #[clap(long)]
    disable_socket: Option<bool>,

    /// Port for the socket (if applicable) to listen on in all subsequent runs
    #[clap(short, long)]
    port: Option<u16>,

    /// Ip for the socket (if applicable) to listen for requests.
    #[clap(long)]
    host: Option<IpAddr>,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl AsRef<CommonClientInitArgs> for Init {
    fn as_ref(&self) -> &CommonClientInitArgs {
        &self.common_args
    }
}

impl From<Init> for OverrideConfig {
    fn from(init_config: Init) -> Self {
        OverrideConfig {
            nym_apis: init_config.common_args.nym_apis,
            disable_socket: init_config.disable_socket,
            port: init_config.port,
            host: init_config.host,
            fastmode: init_config.common_args.fastmode,
            no_cover: init_config.common_args.no_cover,

            nyxd_urls: init_config.common_args.nyxd_urls,
            enabled_credentials_mode: init_config.common_args.enabled_credentials_mode,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InitResults {
    #[serde(flatten)]
    client_core: nym_client_core::init::types::InitResults,
    client_listening_port: u16,
    client_address: String,
}

impl InitResults {
    fn new(res: InitResultsWithConfig<Config>) -> Self {
        Self {
            client_address: res.init_results.address.to_string(),
            client_core: res.init_results,
            client_listening_port: res.config.socket.listening_port,
        }
    }
}

impl Display for InitResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.client_core)?;
        writeln!(f, "Client listening port: {}", self.client_listening_port)?;
        write!(f, "Address of this client: {}", self.client_address)
    }
}

pub(crate) async fn execute(args: Init) -> Result<(), ClientError> {
    eprintln!("Initialising client...");

    let output = args.output;
    let res = initialise_client::<NativeClientInit>(args).await?;

    let init_results = InitResults::new(res);
    println!("{}", output.format(&init_results));

    Ok(())
}
