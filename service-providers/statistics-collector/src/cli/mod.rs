// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::ecash::Ecash;
use clap::{CommandFactory, Parser, Subcommand};
use log::error;
use nym_bin_common::bin_info;
use nym_bin_common::completions::{fig_generate, ArgShell};
use nym_client_core::cli_helpers::CliClient;
use nym_config::OptionalSet;
use nym_statistics_collector::{
    config::{helpers::try_upgrade_config, BaseClientConfig, Config},
    error::StatsCollectorError,
};
use std::path::PathBuf;
use std::sync::OnceLock;

mod add_gateway;
mod build_info;
pub mod ecash;
mod init;
mod list_gateways;
mod run;
mod sign;
mod switch_gateway;

pub(crate) struct CliStatsCollectorClient;

impl CliClient for CliStatsCollectorClient {
    const NAME: &'static str = "statistics-collector";
    type Error = StatsCollectorError;
    type Config = Config;

    async fn try_upgrade_outdated_config(id: &str) -> Result<(), Self::Error> {
        try_upgrade_config(id).await
    }

    async fn try_load_current_config(id: &str) -> Result<Self::Config, Self::Error> {
        try_load_current_config(id).await
    }
}

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser)]
#[command(author = "Nymtech", version, about, long_version = pretty_build_info_static())]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the client.
    #[arg(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Flag used for disabling the printed banner in tty.
    #[arg(long)]
    pub(crate) no_banner: bool,

    #[command(subcommand)]
    command: Commands,
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Initialize a stats collector. Do this first!
    Init(init::Init),

    /// Run the stats collector with the provided configuration and optionally override
    /// parameters.
    Run(run::Run),

    /// Ecash-related functionalities
    Ecash(Ecash),

    /// List all registered with gateways
    ListGateways(list_gateways::Args),

    /// Add new gateway to this client
    AddGateway(add_gateway::Args),

    /// Change the currently active gateway. Note that you must have already registered with the new gateway!
    SwitchGateway(switch_gateway::Args),

    /// Sign to prove ownership of this stats collector
    Sign(sign::Sign),

    /// Show build information of this binary
    BuildInfo(build_info::BuildInfo),

    /// Generate shell completions
    Completions(ArgShell),

    /// Generate Fig specification
    GenerateFigSpec,
}

// Configuration that can be overridden.
pub(crate) struct OverrideConfig {
    nym_apis: Option<Vec<url::Url>>,
    nyxd_urls: Option<Vec<url::Url>>,
    enabled_credentials_mode: Option<bool>,
    report_database_path: Option<PathBuf>,
}

pub(crate) fn override_config(config: Config, args: OverrideConfig) -> Config {
    config
        .with_optional_base_custom_env(
            BaseClientConfig::with_custom_nym_apis,
            args.nym_apis,
            nym_network_defaults::var_names::NYM_API,
            nym_config::parse_urls,
        )
        .with_optional_base_custom_env(
            BaseClientConfig::with_custom_nyxd,
            args.nyxd_urls,
            nym_network_defaults::var_names::NYXD,
            nym_config::parse_urls,
        )
        .with_optional_base(
            BaseClientConfig::with_disabled_credentials,
            args.enabled_credentials_mode.map(|b| !b),
        )
        .with_optional(Config::with_report_database_path, args.report_database_path)
}

pub(crate) async fn execute(args: Cli) -> Result<(), StatsCollectorError> {
    let bin_name = "nym-statistics-collector";

    match args.command {
        Commands::Init(m) => init::execute(m).await?,
        Commands::Run(m) => run::execute(&m).await?,
        Commands::Ecash(ecash) => ecash.execute().await?,
        Commands::ListGateways(args) => list_gateways::execute(args).await?,
        Commands::AddGateway(args) => add_gateway::execute(args).await?,
        Commands::SwitchGateway(args) => switch_gateway::execute(args).await?,
        Commands::Sign(m) => sign::execute(&m).await?,
        Commands::BuildInfo(m) => build_info::execute(m),
        Commands::Completions(s) => s.generate(&mut Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut Cli::command(), bin_name),
    }
    Ok(())
}

async fn try_load_current_config(id: &str) -> Result<Config, StatsCollectorError> {
    // try to load the config as is
    if let Ok(cfg) = Config::read_from_default_path(id) {
        return if !cfg.validate() {
            Err(StatsCollectorError::ConfigValidationFailure)
        } else {
            Ok(cfg)
        };
    }

    // we couldn't load it - try upgrading it from older revisions
    try_upgrade_config(id).await?;

    let config = match Config::read_from_default_path(id) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {id}. Are you sure you have run `init` before? (Error was: {err})");
            return Err(StatsCollectorError::FailedToLoadConfig(id.to_string()));
        }
    };

    if !config.validate() {
        return Err(StatsCollectorError::ConfigValidationFailure);
    }

    Ok(config)
}
