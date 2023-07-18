// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::*;
use nym_issue_credential::utils::block_until_coconut_is_available;
use nym_issue_credential::utils::recover_credentials;

use crate::context::SigningClientWithNyxd;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_config::DEFAULT_DATA_DIR;
use nym_issue_credential::recovery_storage::RecoveryStorage;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::Coin;

#[derive(Debug, Parser)]
pub struct Args {
    /// Home directory of the client that is supposed to use the credential.
    #[clap(long)]
    pub(crate) client_home_directory: std::path::PathBuf,

    /// A mnemonic for the account that buys the credential
    #[clap(long)]
    pub(crate) mnemonic: Option<bip39::Mnemonic>,

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

pub async fn execute(args: Args, client: SigningClientWithNyxd) {
    // we assume the structure of <home-dir>/data
    let data_dir = args.client_home_directory.join(DEFAULT_DATA_DIR);
    let paths = CommonClientPaths::new_default(data_dir);
    let db_path = paths.credentials_database;

    let shared_storage = nym_credential_storage::initialise_persistent_storage(db_path).await;
    let recovery_storage = RecoveryStorage::new(args.recovery_dir).expect("");

    let network_details = NymNetworkDetails::new_from_env();
    let denom = network_details.chain_details.mix_denom.base;
    let amount = Coin::new(args.amount as u128, &denom);

    block_until_coconut_is_available(&client).await.expect("");
    info!("Starting depositing funds, don't kill the process");

    if !args.recovery_mode {
        let state = nym_bandwidth_controller::acquire::deposit(&client.nyxd, amount)
            .await
            .expect("");
        if nym_bandwidth_controller::acquire::get_credential(&state, &client, &shared_storage)
            .await
            .is_err()
        {
            warn!("Failed to obtain credential. Dumping recovery data.",);
            match recovery_storage.insert_voucher(&state.voucher) {
                Ok(file_path) => {
                    warn!("Dumped recovery data to {:?}. Try using recovery mode to convert it to a credential", file_path);
                }
                Err(e) => {
                    error!("Could not dump recovery data to file system due to {:?}, the deposit will be lost!", e)
                }
            }
        }
    } else {
        recover_credentials(&client, &recovery_storage, &shared_storage)
            .await
            .expect("");
    }

    info!(
        "Succeeded adding a credential for {} with amount {}{}",
        args.client_home_directory.to_str().unwrap(),
        &args.amount,
        denom,
    );
}
