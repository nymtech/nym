// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::commands::{
    bonding_information, build_info, debug, migrate, node_details, reset_sphinx_keys, run, sign,
    test_throughput,
};
use crate::env::vars::{NYMNODE_CONFIG_ENV_FILE_ARG, NYMNODE_NO_BANNER_ARG};
// use crate::error::NymNodeError;
use clap::{Args, Parser, Subcommand};
use nym_bin_common::{
    bin_info,
    logging::setup_no_otel_logger,
};
#[cfg(feature = "otel")]
use nym_bin_common::logging::error::TracingError;
#[cfg(feature = "otel")]
use nym_bin_common::opentelemetry::setup_tracing_logger;
#[cfg(feature = "otel")]
use opentelemetry::{global, trace::{TraceContextExt, Tracer}};
#[cfg(feature = "otel")]
use tracing::Instrument;
use std::future::Future;
use std::sync::OnceLock;
use tracing::instrument;

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

    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    fn execute_async<F: Future>(fut: F) -> anyhow::Result<F::Output> {
        Ok(tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(fut))
    }

    #[instrument]
    pub(crate) fn execute(self) -> anyhow::Result<()> {
        // NOTE: `test_throughput` sets up its own logger as it has to include additional layers
        // if !matches!(self.command, Commands::TestThroughput(..)) {
        //     crate::logging::setup_tracing_logger()?;
        // }

        match self.command {
            // Sync commands get logger w. no OTEL
            Commands::BuildInfo(args) => {
                setup_no_otel_logger()?;
                build_info::execute(args)?
            },
            Commands::Migrate(args) => {
                setup_no_otel_logger()?;
                migrate::execute(*args)?
            },
            Commands::Debug(debug) => match debug.command {
                DebugCommands::ResetProvidersGatewayDbs(args) => {
                    let _ = Self::execute_async(debug::reset_providers_dbs::execute(args))?;
                }
            },
            Commands::TestThroughput(args) => {
                // Has its own logging setup
                test_throughput::execute(args)?
            },
            // SigNoz/OTEL run in async context
            Commands::BondingInformation(args) => Self::execute_async(async move {
                #[cfg(feature = "otel")]
                {
                    let _guard = setup_tracing_logger("nym-node".to_string())
                        .map_err(TracingError::from)?;
                    let main_span = tracing::info_span!("startup", service = "nym-node");
                    async {
                        bonding_information::execute(args).in_current_span().await?;
                        Ok::<(), anyhow::Error>(())
                    }
                    .instrument(main_span)
                    .await
                }
                #[cfg(not(feature = "otel"))]
                {
                    setup_no_otel_logger().expect("failed to initialize logging");
                    bonding_information::execute(args).await?;
                    Ok::<(), anyhow::Error>(())
                }
            })??,
            Commands::NodeDetails(args) => Self::execute_async(async move {
                #[cfg(feature = "otel")]
                {
                    let _guard = setup_tracing_logger("nym-node".to_string())
                        .map_err(TracingError::from)?;
                    let main_span = tracing::info_span!("startup", service = "nym-node");
                    async {
                        node_details::execute(args).in_current_span().await?;
                        Ok::<(), anyhow::Error>(())
                    }
                    .instrument(main_span)
                    .await
                }
                #[cfg(not(feature = "otel"))]
                {
                    setup_no_otel_logger().expect("failed to initialize logging");
                    node_details::execute(args).await?;
                    Ok::<(), anyhow::Error>(())
                }
            })??,
            Commands::Run(args) => Self::execute_async(async move {
                #[cfg(feature = "otel")]
                {
                    let _guard = setup_tracing_logger("nym-node".to_string())
                        .map_err(TracingError::from)?;
                    let main_span = tracing::info_span!("startup", service = "nym-node");
                    async {
                        run::execute(*args).in_current_span().await?;
                        Ok::<(), anyhow::Error>(())
                    }
                    .instrument(main_span)
                    .await
                }
                #[cfg(not(feature = "otel"))]
                {
                    setup_no_otel_logger().expect("failed to initialize logging");
                    run::execute(*args).await?;
                    Ok::<(), anyhow::Error>(())
                }
            })??,
            Commands::Sign(args) => Self::execute_async(async move {
                #[cfg(feature = "otel")]
                {
                    let _guard = setup_tracing_logger("nym-node".to_string())
                        .map_err(TracingError::from)?;
                    let main_span = tracing::info_span!("startup", service = "nym-node");
                    async {
                        sign::execute(args).in_current_span().in_current_span().await?;
                        Ok::<(), anyhow::Error>(())
                    }
                    .instrument(main_span)
                    .await
                }
                #[cfg(not(feature = "otel"))]
                {
                    setup_no_otel_logger().expect("failed to initialize logging");
                    sign::execute(args).await?;
                    Ok::<(), anyhow::Error>(())
                }
            })??,
            Commands::UnsafeResetSphinxKeys(args) => Self::execute_async(async move {
                #[cfg(feature = "otel")]
                {
                    let _guard = setup_tracing_logger("nym-node".to_string())
                        .map_err(TracingError::from)?;
                    let main_span = tracing::info_span!("startup", service = "nym-node");
                    async {
                        reset_sphinx_keys::execute(args).in_current_span().await?;
                        Ok::<(), anyhow::Error>(())
                    }
                    .instrument(main_span)
                    .await
                }
                #[cfg(not(feature = "otel"))]
                {
                    setup_no_otel_logger().expect("failed to initialize logging");
                    reset_sphinx_keys::execute(args).await?;
                    Ok::<(), anyhow::Error>(())
                }
            })??,
        }
        Ok(())
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
