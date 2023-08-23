// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use nym_credential_utils::errors::Result;
use nym_credential_utils::utils;

use crate::context::SigningClientWithNyxd;
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

    /// The amount of utokens the credential will hold.
    #[clap(long, default_value = "0")]
    pub(crate) amount: u64,

    /// Path to a directory used to store recovery files for unconsumed deposits
    #[clap(long)]
    pub(crate) recovery_dir: std::path::PathBuf,
}

pub async fn execute(args: Args, client: SigningClientWithNyxd) -> Result<()> {
    let network_details = NymNetworkDetails::new_from_env();
    let coin = Coin::new(
        args.amount as u128,
        network_details.chain_details.mix_denom.base,
    );

    let persistent_storage = utils::setup_persistent_storage(args.client_home_directory).await;
    utils::issue_credential(&client.nyxd, coin, &persistent_storage, args.recovery_dir).await?;

    Ok(())
}
