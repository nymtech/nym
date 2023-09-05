// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::{BackendError, Result};
use bip39::Mnemonic;
use nym_cli_commands::coconut::issue_credentials::Args as CoconutArgs;
use nym_cli_commands::context::{create_signing_client, get_network_details, ClientArgs};
use std::path::PathBuf;
use url::Url;

pub async fn handle_url(url: &str) -> Result<()> {
    let url = Url::parse(url)?;
    if url.scheme() != env!("CARGO_PKG_NAME") {
        return Err(BackendError::InvalidURLScheme {
            scheme: url.scheme().to_string(),
        });
    }
    let bytes = bs58::decode(url.path()).into_vec()?;
    let mnemonic = Mnemonic::from_entropy(&bytes)?;

    let args = ClientArgs {
        config_env_file: None,
        nyxd_url: None,
        nym_api_url: None,
        mnemonic: Some(mnemonic),
        mixnet_contract_address: None,
        vesting_contract_address: None,
    };
    let network_details = get_network_details(&args)?;
    let client = create_signing_client(args, &network_details)?;
    let coconut_args = CoconutArgs {
        client_config: Default::default(),
        amount: 10,
        recovery_dir: PathBuf::default(),
    };
    nym_cli_commands::coconut::issue_credentials::execute(coconut_args, client).await?;

    Ok(())
}
