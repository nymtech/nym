// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::{override_config, CliAuthenticatorClient, OverrideConfig};
use clap::Args;
use nym_authenticator::{
    config::{default_config_directory, default_config_filepath, default_data_directory, Config},
    error::AuthenticatorError,
};
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::cli_helpers::client_init::{
    initialise_client, CommonClientInitArgs, InitResultsWithConfig, InitialisableClient,
};
use serde::Serialize;
use std::{fmt::Display, fs, path::PathBuf};

impl InitialisableClient for CliAuthenticatorClient {
    type InitArgs = Init;

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

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl From<Init> for OverrideConfig {
    fn from(init_config: Init) -> Self {
        OverrideConfig {
            nym_apis: init_config.common_args.nym_apis,
            nyxd_urls: init_config.common_args.nyxd_urls,
            enabled_credentials_mode: init_config.common_args.enabled_credentials_mode,
        }
    }
}

impl AsRef<CommonClientInitArgs> for Init {
    fn as_ref(&self) -> &CommonClientInitArgs {
        &self.common_args
    }
}

#[derive(Debug, Serialize)]
pub struct InitResults {
    #[serde(flatten)]
    client_core: nym_client_core::init::types::InitResults,
    client_address: String,
}

impl InitResults {
    fn new(res: InitResultsWithConfig<Config>) -> Self {
        Self {
            client_address: res.init_results.address.to_string(),
            client_core: res.init_results,
        }
    }
}

impl Display for InitResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.client_core)?;
        write!(f, "Address of this authenticator: {}", self.client_address)
    }
}

pub(crate) async fn execute(args: Init) -> Result<(), AuthenticatorError> {
    eprintln!("Initialising client...");

    let output = args.output;
    let res = initialise_client::<CliAuthenticatorClient>(args, None).await?;

    let init_results = InitResults::new(res);
    println!("{}", output.format(&init_results));

    Ok(())
}
