// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{ArgGroup, Args, Subcommand};
use log::*;
use nym_bandwidth_controller::acquire::state::State;
use nym_bin_common::completions::ArgShell;
use nym_coconut_interface::Parameters;
use nym_credential_storage::persistent_storage::PersistentStorage;
use nym_credentials::coconut::bandwidth::TOTAL_ATTRIBUTES;
use nym_validator_client::nyxd::traits::DkgQueryClient;

use crate::error::Result;
use crate::recovery_storage::RecoveryStorage;

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

pub(crate) async fn recover_credentials<C: DkgQueryClient + Send + Sync>(
    client: &C,
    recovery_storage: &RecoveryStorage,
    shared_storage: &PersistentStorage,
) -> Result<()> {
    for voucher in recovery_storage.unconsumed_vouchers()? {
        let state = State {
            voucher,
            params: Parameters::new(TOTAL_ATTRIBUTES).unwrap(),
        };
        if let Err(e) =
            nym_bandwidth_controller::acquire::get_credential(&state, client, shared_storage).await
        {
            error!(
                "Could not recover deposit {} due to {:?}, try again later",
                state.voucher.tx_hash(),
                e
            )
        } else {
            info!(
                "Converted deposit {} to a credential, removing recovery data for it",
                state.voucher.tx_hash()
            );
            if let Err(e) = recovery_storage.remove_voucher(state.voucher.tx_hash().to_string()) {
                warn!("Could not remove recovery data - {:?}", e);
            }
        }
    }

    Ok(())
}
