// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::CliSocks5Client;
use crate::config::{
    default_config_directory, default_config_filepath, default_data_directory, Config,
};
use crate::{
    commands::{override_config, OverrideConfig},
    error::Socks5ClientError,
};
use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::cli_helpers::client_init::{
    initialise_client, CommonClientInitArgs, InitResultsWithConfig, InitialisableClient,
};
use nym_sphinx::addressing::clients::Recipient;
use serde::Serialize;
use std::fmt::Display;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

impl InitialisableClient for CliSocks5Client {
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
            Config::new(&init_args.common_args.id, &init_args.provider.to_string()),
            OverrideConfig::from(init_args.clone()),
        )
    }
}

#[derive(Args, Clone, Debug)]
pub(crate) struct Init {
    #[command(flatten)]
    common_args: CommonClientInitArgs,

    /// Address of the socks5 provider to send messages to.
    #[clap(long)]
    provider: Recipient,

    /// Specifies whether this client is going to use an anonymous sender tag for communication with the service provider.
    /// While this is going to hide its actual address information, it will make the actual communication
    /// slower and consume nearly double the bandwidth as it will require sending reply SURBs.
    ///
    /// Note that some service providers might not support this.
    // the alias here is included for backwards compatibility (1.1.4 and before)
    #[clap(long, alias = "use_anonymous_sender_tag")]
    use_reply_surbs: Option<bool>,

    /// Port for the socket to listen on in all subsequent runs
    #[clap(short, long)]
    port: Option<u16>,

    /// The custom host on which the socks5 client will be listening for requests
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
            ip: init_config.host,
            port: init_config.port,
            use_anonymous_replies: init_config.use_reply_surbs,
            fastmode: init_config.common_args.fastmode,
            no_cover: init_config.common_args.no_cover,
            geo_routing: None,
            medium_toggle: false,
            nyxd_urls: init_config.common_args.nyxd_urls,
            enabled_credentials_mode: init_config.common_args.enabled_credentials_mode,
            outfox: false,
            stats_reporting_address: init_config.common_args.stats_reporting_address,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InitResults {
    #[serde(flatten)]
    client_core: nym_client_core::init::types::InitResults,
    socks5_listening_address: SocketAddr,
    client_address: String,
}

impl InitResults {
    fn new(res: InitResultsWithConfig<Config>) -> Self {
        Self {
            client_address: res.init_results.address.to_string(),
            client_core: res.init_results,
            socks5_listening_address: res.config.core.socks5.bind_address,
        }
    }
}

impl Display for InitResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.client_core)?;
        writeln!(
            f,
            "SOCKS5 listening address: {}",
            self.socks5_listening_address
        )?;
        write!(f, "Address of this client: {}", self.client_address)
    }
}

pub(crate) async fn execute(args: Init) -> Result<(), Socks5ClientError> {
    eprintln!("Initialising client...");

    let user_agent = nym_bin_common::bin_info!().into();
    let output = args.output;
    let res = initialise_client::<CliSocks5Client>(args, Some(user_agent)).await?;

    let init_results = InitResults::new(res);
    println!("{}", output.format(&init_results));

    Ok(())
}
