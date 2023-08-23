// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{ArgGroup, Args, Subcommand};
use nym_bin_common::completions::ArgShell;

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Run the binary to obtain a credential
    Run(Run),

    /// Generate shell completions
    Completions(ArgShell),

    /// Generate Fig specification
    GenerateFigSpec,
}

#[derive(Args)]
#[clap(group(
ArgGroup::new("recov")
.required(true)
.args(&["amount", "recovery_mode"]),
))]
pub(crate) struct Run {
    /// Home directory of the client that is supposed to use the credential.
    #[clap(long)]
    pub(crate) client_home_directory: std::path::PathBuf,

    /// A mnemonic for the account that buys the credential
    #[clap(long)]
    pub(crate) mnemonic: String,

    /// The amount of utokens the credential will hold. If recovery mode is enabled, this value
    /// is not needed
    #[clap(long, default_value = "0")]
    pub(crate) amount: u64,

    /// Path to a directory used to store recovery files for unconsumed deposits
    #[clap(long)]
    pub(crate) recovery_dir: std::path::PathBuf,

    /// Recovery mode, when enabled, tries to recover any deposit data dumped in recovery_dir
    #[clap(long)]
    pub(crate) recovery_mode: bool,
}
