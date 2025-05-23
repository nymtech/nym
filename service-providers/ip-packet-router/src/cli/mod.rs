use crate::cli::ecash::Ecash;
use clap::{CommandFactory, Parser, Subcommand};
use log::error;
use nym_bin_common::bin_info;
use nym_bin_common::completions::{fig_generate, ArgShell};
use nym_client_core::cli_helpers::CliClient;
use nym_ip_packet_router::config::helpers::{try_upgrade_config, try_upgrade_config_by_id};
use nym_ip_packet_router::config::{BaseClientConfig, Config};
use nym_ip_packet_router::error::IpPacketRouterError;
use std::sync::OnceLock;

mod add_gateway;
mod build_info;
pub mod ecash;
mod init;
mod list_gateways;
mod run;
mod sign;
mod switch_gateway;

pub(crate) struct CliIpPacketRouterClient;

impl CliClient for CliIpPacketRouterClient {
    const NAME: &'static str = "ip packet router";
    type Error = IpPacketRouterError;
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
    /// Initialize a network-requester. Do this first!
    Init(init::Init),

    /// Run the network requester with the provided configuration and optionally override
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

    /// Sign to prove ownership of this network requester
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
}

pub(crate) fn override_config(mut config: Config, args: OverrideConfig) -> Config {
    // disable poisson rate in the BASE client if the IPR option is enabled
    if config.ip_packet_router.disable_poisson_rate {
        log::info!("Disabling poisson rate for ip packet router");
        config.set_no_poisson_process();
    }

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
}

pub(crate) async fn execute(args: Cli) -> Result<(), IpPacketRouterError> {
    let bin_name = "nym-ip-packet-router";

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

async fn try_load_current_config(id: &str) -> Result<Config, IpPacketRouterError> {
    // try to load the config as is
    if let Ok(cfg) = Config::read_from_default_path(id) {
        return if !cfg.validate() {
            Err(IpPacketRouterError::ConfigValidationFailure)
        } else {
            Ok(cfg)
        };
    }

    // we couldn't load it - try upgrading it from older revisions
    try_upgrade_config_by_id(id).await?;

    let config = match Config::read_from_default_path(id) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {id}. Are you sure you have run `init` before? (Error was: {err})");
            return Err(IpPacketRouterError::FailedToLoadConfig(id.to_string()));
        }
    };

    if !config.validate() {
        return Err(IpPacketRouterError::ConfigValidationFailure);
    }

    Ok(config)
}
