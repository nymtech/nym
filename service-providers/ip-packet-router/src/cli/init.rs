use std::{fmt::Display, fs, path::PathBuf};

use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::cli_helpers::client_init::{
    initialise_client, CommonClientInitArgs, InitResultsWithConfig, InitialisableClient,
};
use serde::Serialize;

use crate::{
    cli::{override_config, try_upgrade_config, OverrideConfig},
    config::{default_config_directory, default_config_filepath, default_data_directory, Config},
    error::IpPacketRouterError,
};

struct IpPacketRouterInit;

impl InitialisableClient for IpPacketRouterInit {
    const NAME: &'static str = "ip packet router";
    type Error = IpPacketRouterError;
    type InitArgs = Init;
    type Config = Config;

    fn try_upgrade_outdated_config(id: &str) -> Result<(), Self::Error> {
        try_upgrade_config(id)
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

#[derive(Args, Clone)]
pub(crate) struct Init {
    #[command(flatten)]
    common_args: CommonClientInitArgs,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl From<Init> for OverrideConfig {
    fn from(_init_config: Init) -> Self {
        OverrideConfig {
            // nym_apis: init_config.common_args.nym_apis,
            // fastmode: init_config.common_args.fastmode,
            // no_cover: init_config.common_args.no_cover,
            // nyxd_urls: init_config.common_args.nyxd_urls,
            // enabled_credentials_mode: init_config.common_args.enabled_credentials_mode,
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
        write!(
            f,
            "Address of this ip-packet-router: {}",
            self.client_address
        )
    }
}

pub(crate) async fn execute(args: Init) -> Result<(), IpPacketRouterError> {
    eprintln!("Initialising client...");

    let output = args.output;
    let res = initialise_client::<IpPacketRouterInit>(args).await?;

    let init_results = InitResults::new(res);
    println!("{}", output.format(&init_results));

    Ok(())
}
