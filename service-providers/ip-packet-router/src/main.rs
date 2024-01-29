use clap::{crate_name, crate_version, Parser};
use error::IpPacketRouterError;
use nym_bin_common::logging::{maybe_print_banner, setup_logging};
use nym_network_defaults::setup_env;

use crate::config::Config;

mod cli;
mod config;
mod constants;
mod error;
mod ip_packet_router;
mod mixnet_client;
mod mixnet_listener;
mod request_filter;
mod tun_listener;
mod util;

#[tokio::main]
async fn main() -> Result<(), IpPacketRouterError> {
    let args = cli::Cli::parse();
    setup_logging();
    setup_env(args.config_env_file.as_ref());

    if !args.no_banner {
        maybe_print_banner(crate_name!(), crate_version!());
    }

    cli::execute(args).await
}
