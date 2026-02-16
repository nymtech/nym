// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::commands::{
    bonding_information, build_info, debug, migrate, node_details, reset_sphinx_keys, run, sign,
    test_throughput,
};
use crate::env::vars::{NYMNODE_CONFIG_ENV_FILE_ARG, NYMNODE_NO_BANNER_ARG};
use clap::{Args, Parser, Subcommand};
use nym_bin_common::bin_info;
use std::sync::OnceLock;

pub(crate) mod commands;
mod helpers;

pub const DEFAULT_NYMNODE_ID: &str = "default-nym-node";

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the nym-node and overrides any preconfigured values.
    #[clap(
        short,
        long,
        env = NYMNODE_CONFIG_ENV_FILE_ARG
    )]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Flag used for disabling the printed banner in tty.
    #[clap(
        long,
        env = NYMNODE_NO_BANNER_ARG
    )]
    pub(crate) no_banner: bool,

    /// Enable OpenTelemetry tracing export via OTLP/gRPC.
    /// Requires the binary to be built with the `otel` feature.
    #[clap(long, env = "NYMNODE_OTEL_ENABLE")]
    pub(crate) otel: bool,

    /// OpenTelemetry OTLP collector endpoint (gRPC).
    /// Only used when --otel is enabled.
    /// For SigNoz Cloud use https://ingest.<region>.signoz.cloud:443
    #[clap(
        long,
        env = "NYMNODE_OTEL_ENDPOINT",
        default_value = "http://localhost:4317"
    )]
    pub(crate) otel_endpoint: String,

    /// SigNoz Cloud ingestion key for authenticated OTLP export.
    /// Only needed for SigNoz Cloud (not self-hosted).
    #[clap(long, env = "NYMNODE_OTEL_KEY")]
    pub(crate) otel_key: Option<String>,

    /// Deployment environment label attached to all exported traces.
    /// Used to distinguish sandbox / mainnet / canary in the OTel backend.
    #[clap(long, env = "NYMNODE_OTEL_ENV", default_value = "mainnet")]
    pub(crate) otel_env: String,

    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub(crate) fn execute(self) -> anyhow::Result<()> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        // Set up tracing inside the runtime so the OTel batch exporter
        // can spawn its background tasks on the tokio reactor.
        let _otel_guard = runtime.block_on(async { self.setup_logging() })?;

        // `_otel_guard` is dropped at function exit, flushing pending spans via its Drop impl
        runtime.block_on(async {
            match self.command {
                Commands::BuildInfo(args) => build_info::execute(args)?,
                Commands::BondingInformation(args) => bonding_information::execute(args).await?,
                Commands::NodeDetails(args) => node_details::execute(args).await?,
                Commands::Run(args) => run::execute(*args).await?,
                Commands::Migrate(args) => migrate::execute(*args)?,
                Commands::Sign(args) => sign::execute(args).await?,
                Commands::TestThroughput(args) => test_throughput::execute(args)?,
                Commands::UnsafeResetSphinxKeys(args) => reset_sphinx_keys::execute(args).await?,
                Commands::Debug(debug) => match debug.command {
                    DebugCommands::ResetProvidersGatewayDbs(args) => {
                        debug::reset_providers_dbs::execute(args).await?
                    }
                },
            }
            Ok::<(), anyhow::Error>(())
        })
    }

    #[cfg(feature = "otel")]
    fn setup_logging(&self) -> anyhow::Result<Option<crate::logging::OtelGuard>> {
        if matches!(self.command, Commands::TestThroughput(..)) {
            return Ok(None);
        }
        let otel_config = if self.otel {
            Some(crate::logging::OtelConfig {
                endpoint: self.otel_endpoint.clone(),
                service_name: "nym-node".to_string(),
                ingestion_key: self.otel_key.clone(),
                environment: self.otel_env.clone(),
            })
        } else {
            None
        };
        crate::logging::setup_tracing_logger(otel_config)
    }

    #[cfg(not(feature = "otel"))]
    fn setup_logging(&self) -> anyhow::Result<Option<()>> {
        if matches!(self.command, Commands::TestThroughput(..)) {
            return Ok(None);
        }
        let otel_config = if self.otel {
            Some(crate::logging::OtelConfig {
                endpoint: self.otel_endpoint.clone(),
                service_name: "nym-node".to_string(),
                ingestion_key: self.otel_key.clone(),
                environment: self.otel_env.clone(),
            })
        } else {
            None
        };
        crate::logging::setup_tracing_logger(otel_config)?;
        Ok(None)
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Show build information of this binary
    BuildInfo(build_info::Args),

    /// Show bonding information of this node depending on its currently selected mode.
    BondingInformation(bonding_information::Args),

    /// Show details of this node.
    NodeDetails(node_details::Args),

    /// Attempt to migrate an existing mixnode or gateway into a nym-node.
    Migrate(Box<migrate::Args>),

    /// Start this nym-node
    Run(Box<run::Args>),

    /// Use identity key of this node to sign provided message.
    Sign(sign::Args),

    /// UNSAFE: reset existing sphinx keys and attempt to generate fresh one for the current network state
    UnsafeResetSphinxKeys(reset_sphinx_keys::Args),

    /// Commands exposed for debug purposes, usually not meant to be used by operators
    #[clap(hide = true)]
    Debug(Debug),

    /// Attempt to approximate the maximum mixnet throughput if nym-node
    /// was running on this machine in mixnet mode
    #[clap(hide = true)]
    TestThroughput(test_throughput::Args),
}

#[derive(Debug, Args)]
pub(crate) struct Debug {
    #[clap(subcommand)]
    pub(crate) command: DebugCommands,
}

#[derive(Subcommand, Debug)]
pub(crate) enum DebugCommands {
    /// Reset the internal GatewaysDetailsStores of all service providers in case they got corrupted
    ResetProvidersGatewayDbs(debug::reset_providers_dbs::Args),
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
