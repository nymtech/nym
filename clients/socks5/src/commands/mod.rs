// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::old_config_v1_1_13::OldConfigV1_1_13;
use crate::config::old_config_v1_1_20::ConfigV1_1_20;
use crate::config::old_config_v1_1_20_2::ConfigV1_1_20_2;
use crate::config::old_config_v1_1_30::ConfigV1_1_30;
use crate::config::old_config_v1_1_33::ConfigV1_1_33;
use crate::config::{BaseClientConfig, Config};
use crate::error::Socks5ClientError;
use clap::CommandFactory;
use clap::{Parser, Subcommand};
use log::{error, info};
use nym_bin_common::bin_info;
use nym_bin_common::completions::{fig_generate, ArgShell};
use nym_client_core::cli_helpers::client_import_credential::CommonClientImportCredentialArgs;
use nym_client_core::cli_helpers::CliClient;
use nym_client_core::client::base_client::storage::migration_helpers::v1_1_33;
use nym_client_core::client::topology_control::geo_aware_provider::CountryGroup;
use nym_client_core::config::{GroupBy, TopologyStructure};
use nym_config::OptionalSet;
use nym_sphinx::params::{PacketSize, PacketType};
use std::error::Error;
use std::net::IpAddr;
use std::sync::OnceLock;

mod add_gateway;
pub(crate) mod build_info;
mod import_credential;
pub mod init;
mod list_gateways;
pub(crate) mod run;
mod show_ticketbooks;
mod switch_gateway;

pub(crate) struct CliSocks5Client;

impl CliClient for CliSocks5Client {
    const NAME: &'static str = "socks5";
    type Error = Socks5ClientError;
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
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the client.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Flag used for disabling the printed banner in tty.
    #[clap(long)]
    pub(crate) no_banner: bool,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Initialise a Nym client. Do this first!
    Init(init::Init),

    /// Run the Nym client with provided configuration client optionally overriding set parameters
    Run(run::Run),

    /// Import a pre-generated credential
    ImportCredential(CommonClientImportCredentialArgs),

    /// List all registered with gateways
    ListGateways(list_gateways::Args),

    /// Add new gateway to this client
    AddGateway(add_gateway::Args),

    /// Change the currently active gateway. Note that you must have already registered with the new gateway!
    SwitchGateway(switch_gateway::Args),

    /// Display information associated with the imported ticketbooks,
    ShowTicketbooks(show_ticketbooks::Args),

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
    ip: Option<IpAddr>,
    port: Option<u16>,
    use_anonymous_replies: Option<bool>,
    fastmode: bool,
    no_cover: bool,
    geo_routing: Option<CountryGroup>,
    medium_toggle: bool,
    nyxd_urls: Option<Vec<url::Url>>,
    enabled_credentials_mode: Option<bool>,
    outfox: bool,
}

pub(crate) async fn execute(args: Cli) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bin_name = "nym-socks5-client";

    match args.command {
        Commands::Init(m) => init::execute(m).await?,
        Commands::Run(m) => run::execute(m).await?,
        Commands::ImportCredential(m) => import_credential::execute(m).await?,
        Commands::ListGateways(args) => list_gateways::execute(args).await?,
        Commands::AddGateway(args) => add_gateway::execute(args).await?,
        Commands::SwitchGateway(args) => switch_gateway::execute(args).await?,
        Commands::ShowTicketbooks(args) => show_ticketbooks::execute(args).await?,
        Commands::BuildInfo(m) => build_info::execute(m),
        Commands::Completions(s) => s.generate(&mut Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut Cli::command(), bin_name),
    }
    Ok(())
}

pub(crate) fn override_config(config: Config, args: OverrideConfig) -> Config {
    let disable_cover_traffic_with_keepalive = args.medium_toggle;
    let secondary_packet_size = args.medium_toggle.then_some(PacketSize::ExtendedPacket16);
    let no_per_hop_delays = args.medium_toggle;

    let topology_structure = if args.medium_toggle {
        // Use the location of the network-requester
        let address = config
            .core
            .socks5
            .provider_mix_address
            .parse()
            .expect("failed to parse provider mix address");
        TopologyStructure::GeoAware(GroupBy::NymAddress(address))
    } else if let Some(code) = args.geo_routing {
        TopologyStructure::GeoAware(GroupBy::CountryGroup(code))
    } else {
        TopologyStructure::default()
    };

    let packet_type = if args.outfox {
        PacketType::Outfox
    } else {
        PacketType::Mix
    };
    config
        .with_base(
            BaseClientConfig::with_high_default_traffic_volume,
            args.fastmode,
        )
        .with_base(
            // NOTE: This interacts with disabling cover traffic fully, so we want to this to be set before
            BaseClientConfig::with_disabled_cover_traffic_with_keepalive,
            disable_cover_traffic_with_keepalive,
        )
        .with_base(
            BaseClientConfig::with_secondary_packet_size,
            secondary_packet_size,
        )
        .with_base(BaseClientConfig::with_no_per_hop_delays, no_per_hop_delays)
        // NOTE: see comment above about the order of the other disble cover traffic config
        .with_base(BaseClientConfig::with_disabled_cover_traffic, args.no_cover)
        .with_base(BaseClientConfig::with_packet_type, packet_type)
        .with_base(
            BaseClientConfig::with_topology_structure,
            topology_structure,
        )
        .with_optional(Config::with_anonymous_replies, args.use_anonymous_replies)
        .with_optional(Config::with_port, args.port)
        .with_optional(Config::with_ip, args.ip)
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

async fn try_upgrade_v1_1_13_config(id: &str) -> Result<bool, Socks5ClientError> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.13 (which is incompatible with the next step, i.e. 1.1.19)
    let Ok(old_config) = OldConfigV1_1_13::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.13 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV1_1_20 = old_config.into();
    let updated_step2: ConfigV1_1_20_2 = updated_step1.into();
    let (updated_step3, gateway_config) = updated_step2.upgrade()?;
    let old_paths = updated_step3.storage_paths.clone();

    let updated_step4: ConfigV1_1_33 = updated_step3.into();
    let updated = updated_step4.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        Some(gateway_config),
    )
    .await?;

    updated.save_to_default_location()?;
    Ok(true)
}

async fn try_upgrade_v1_1_20_config(id: &str) -> Result<bool, Socks5ClientError> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.20 (which is incompatible with the current one, i.e. +1.1.21)
    let Ok(old_config) = ConfigV1_1_20::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.20 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV1_1_20_2 = old_config.into();
    let (updated_step2, gateway_config) = updated_step1.upgrade()?;
    let old_paths = updated_step2.storage_paths.clone();

    let updated_step3: ConfigV1_1_33 = updated_step2.into();
    let updated = updated_step3.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        Some(gateway_config),
    )
    .await?;

    updated.save_to_default_location()?;
    Ok(true)
}

async fn try_upgrade_v1_1_20_2_config(id: &str) -> Result<bool, Socks5ClientError> {
    // explicitly load it as v1.1.20_2 (which is incompatible with the current one, i.e. +1.1.21)
    let Ok(old_config) = ConfigV1_1_20_2::read_from_default_path(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.20_2 config template.");
    info!("It is going to get updated to the current specification.");

    let (updated_step1, gateway_config) = old_config.upgrade()?;
    let old_paths = updated_step1.storage_paths.clone();

    let updated_step2: ConfigV1_1_33 = updated_step1.into();
    let updated = updated_step2.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        Some(gateway_config),
    )
    .await?;

    updated.save_to_default_location()?;
    Ok(true)
}

async fn try_upgrade_v1_1_30_config(id: &str) -> Result<bool, Socks5ClientError> {
    // explicitly load it as v1.1.30 (which is incompatible with the current one, i.e. +1.1.31)
    let Ok(old_config) = ConfigV1_1_30::read_from_default_path(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.30 config template.");
    info!("It is going to get updated to the current specification.");

    let old_paths = old_config.storage_paths.clone();

    let updated_step1: ConfigV1_1_33 = old_config.into();
    let updated = updated_step1.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        None,
    )
    .await?;

    updated.save_to_default_location()?;
    Ok(true)
}

async fn try_upgrade_v1_1_33_config(id: &str) -> Result<bool, Socks5ClientError> {
    // explicitly load it as v1.1.33 (which is incompatible with the current one, i.e. +1.1.34)
    let Ok(old_config) = ConfigV1_1_33::read_from_default_path(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.33 config template.");
    info!("It is going to get updated to the current specification.");

    let old_paths = old_config.storage_paths.clone();

    let updated = old_config.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        None,
    )
    .await?;

    updated.save_to_default_location()?;
    Ok(true)
}

async fn try_upgrade_config(id: &str) -> Result<(), Socks5ClientError> {
    if try_upgrade_v1_1_13_config(id).await? {
        return Ok(());
    }
    if try_upgrade_v1_1_20_config(id).await? {
        return Ok(());
    }
    if try_upgrade_v1_1_20_2_config(id).await? {
        return Ok(());
    }
    if try_upgrade_v1_1_30_config(id).await? {
        return Ok(());
    }
    if try_upgrade_v1_1_33_config(id).await? {
        return Ok(());
    }

    Ok(())
}

async fn try_load_current_config(id: &str) -> Result<Config, Socks5ClientError> {
    // try to load the config as is
    if let Ok(cfg) = Config::read_from_default_path(id) {
        return if !cfg.validate() {
            Err(Socks5ClientError::ConfigValidationFailure)
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
            return Err(Socks5ClientError::FailedToLoadConfig(id.to_string()));
        }
    };

    if !config.validate() {
        return Err(Socks5ClientError::ConfigValidationFailure);
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
