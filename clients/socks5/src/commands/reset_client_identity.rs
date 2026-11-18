// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::CliSocks5Client;
use crate::error::Socks5ClientError;
use nym_client_core::cli_helpers::client_reset_client_identity::{
    reset_client_identity, CommonClientResetClientIdentityArgs,
};
use nym_client_core::cli_helpers::database_key::DatabaseKeyArgs;
use nym_client_core::cli_helpers::key_passphrase::KeyPassphraseArgs;

#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    common_args: CommonClientResetClientIdentityArgs,

    #[command(flatten)]
    key_passphrase: KeyPassphraseArgs,

    #[command(flatten)]
    database_key: DatabaseKeyArgs,
}

impl AsRef<CommonClientResetClientIdentityArgs> for Args {
    fn as_ref(&self) -> &CommonClientResetClientIdentityArgs {
        &self.common_args
    }
}

pub(crate) async fn execute(mut args: Args) -> Result<(), Socks5ClientError> {
    let key_passphrase = args.key_passphrase.key_passphrase.take();
    reset_client_identity::<CliSocks5Client, _>(
        &args,
        key_passphrase,
        args.database_key.database_key.as_ref(),
    )
    .await?;

    println!(
        "client '{}' has a new identity and no gateway; run `add-gateway` before running it",
        args.common_args.id
    );
    Ok(())
}
