// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use nym_bin_common::bin_info;
use nym_validator_client::nyxd::AccountId;
use url::Url;

lazy_static::lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String = bin_info!().pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

#[derive(Parser, Debug)]
#[command(author = "Nymtech", version, about, long_version = pretty_build_info_static())]
pub struct Args {
    /// Path pointing to an env file that configures the environment.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(long)]
    pub(crate) admin_mnemonic: bip39::Mnemonic,

    #[clap(long)]
    pub(crate) dkg_contract_address: Option<AccountId>,

    #[clap(long)]
    pub(crate) nyxd_validator: Option<Url>,
}
