// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

use clap::{crate_name, crate_version, Parser};
use colored::Colorize;
use lazy_static::lazy_static;
use log::error;
use nym_bin_common::logging::{maybe_print_banner, setup_logging};
use nym_bin_common::output_format::OutputFormat;
use nym_bin_common::{bin_info, bin_info_owned};
use nym_network_defaults::setup_env;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

mod commands;
mod config;
pub(crate) mod error;
mod http;
mod node;
pub(crate) mod support;

lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String = bin_info!().pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, about, long_version = pretty_build_info_static())]
struct Cli {
    /// Path pointing to an env file that configures the gateway.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Flag used for disabling the printed banner in tty.
    #[clap(long)]
    pub(crate) no_banner: bool,

    #[clap(subcommand)]
    command: commands::Commands,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    setup_logging();

    let config = nym_node::http::Config {
        landing: Default::default(),
        policy: Default::default(),
        api: nym_node::http::api::Config {
            v1_config: nym_node::http::api::v1::Config {
                build_information: bin_info_owned!(),
                gateway: Default::default(),
                mixnide: Default::default(),
                network_requester: Default::default(),
            },
        },
    };
    let mut router = nym_node::http::NymNodeRouter::new(config);

    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 12345);
    let server = router.build_server(&address)?;
    server.await?;

    Ok(())

    //
    //
    // let args = Cli::parse();
    // setup_env(args.config_env_file.as_ref());
    //
    // if !args.no_banner {
    //     maybe_print_banner(crate_name!(), crate_version!());
    // }
    // setup_logging();
    //
    // commands::execute(args).await.map_err(|err| {
    //     if atty::is(atty::Stream::Stdout) {
    //         let error_message = format!("{err}").red();
    //         error!("{error_message}");
    //         error!("Exiting...");
    //     }
    //     err
    // })
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
