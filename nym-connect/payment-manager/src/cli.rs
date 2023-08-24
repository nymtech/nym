// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use lazy_static::lazy_static;
use nym_bin_common::bin_info;

lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String = bin_info!().pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct CliArgs {
    /// Path pointing to an env file that configures the Payment Manager.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Mnemonic of the Nym account holding the funds to be converted after successful payments
    #[clap(short, long)]
    pub(crate) mnemonic: bip39::Mnemonic,

    /// Path pointing to the SQLite database containing payment data.
    #[clap(short, long)]
    pub(crate) db_path: std::path::PathBuf,
}
