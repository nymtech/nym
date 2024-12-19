use clap::{crate_name, crate_version, Parser};
use nym_bin_common::bin_info_owned;
use nym_bin_common::logging::maybe_print_banner;
use nym_network_defaults::setup_env;
use tracing::info;

mod chain_scraper;
mod cli;
mod config;
mod db;
mod env;
mod error;
mod http;
mod logging;
pub mod models;
mod payment_listener;
mod price_scraper;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    setup_env(cli.config_env_file.as_ref());
    logging::setup_tracing_logger();

    if !cli.no_banner {
        maybe_print_banner(crate_name!(), crate_version!());
    }

    let bin_info = bin_info_owned!();
    info!("using the following version: {bin_info}");

    cli.execute().await?;

    Ok(())
}
