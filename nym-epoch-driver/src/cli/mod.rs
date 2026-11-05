// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use url::Url;

mod build_info;
mod init;
mod run;

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the network the driver runs against.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<PathBuf>,

    /// Flag used for disabling the printed banner in tty.
    #[clap(long)]
    pub(crate) no_banner: bool,

    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        match self.command {
            Commands::Init(args) => init::execute(args),
            Commands::Run(args) => run::execute(args).await,
            Commands::BuildInfo(args) => build_info::execute(args),
        }
    }
}

#[derive(clap::Args, Debug)]
pub(crate) struct ConfigOverridableArgs {
    /// Endpoint to the nyxd instance the driver queries and sends its transactions through.
    /// default: the config file's value, or at `init` the network environment's endpoint
    #[clap(long, env = "NYM_EPOCH_DRIVER_NYXD_URL")]
    pub(crate) nyxd_url: Option<Url>,

    /// Socket address the metrics listener binds to.
    #[clap(long, env = "NYM_EPOCH_DRIVER_METRICS_BIND_ADDRESS")]
    pub(crate) metrics_bind_address: Option<SocketAddr>,

    /// How often the chain tip is read for a newly settled cadence boundary.
    #[clap(
        long,
        hide = true,
        value_parser = humantime::parse_duration,
        env = "NYM_EPOCH_DRIVER_TIP_POLLING_INTERVAL"
    )]
    pub(crate) tip_polling_interval: Option<Duration>,

    /// Number of blocks a cadence boundary must lie below the tip before the network view is
    /// produced at it.
    #[clap(long, hide = true, env = "NYM_EPOCH_DRIVER_SETTLE_LAG")]
    pub(crate) settle_lag: Option<u64>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialise the epoch driver with a persistent config.toml file.
    Init(init::Args),

    /// Run the epoch driver with its persistent configuration.
    Run(run::Args),

    /// Show build information of this binary
    BuildInfo(build_info::Args),
}
