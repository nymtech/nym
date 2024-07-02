// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;

    let args = cli::Cli::parse();
    nym_bin_common::logging::setup_logging();
    nym_network_defaults::setup_env(args.config_env_file.as_ref());

    if !args.no_banner {
        nym_bin_common::logging::maybe_print_banner(clap::crate_name!(), clap::crate_version!());
    }

    cli::execute(args).await?;
    Ok(())
}
