// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

#[derive(Parser)]
#[clap(author = "Nymtech", version, about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the explorer api.
    #[clap(long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,
}
