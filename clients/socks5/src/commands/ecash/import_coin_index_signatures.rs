// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::CliSocks5Client;
use crate::error::Socks5ClientError;
use nym_client_core::cli_helpers::client_import_coin_index_signatures::{
    import_coin_index_signatures, CommonClientImportCoinIndexSignaturesArgs,
};
use nym_client_core::cli_helpers::database_key::DatabaseKeyArgs;

#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    common_args: CommonClientImportCoinIndexSignaturesArgs,

    #[command(flatten)]
    database_key: DatabaseKeyArgs,
}

pub(crate) async fn execute(args: Args) -> Result<(), Socks5ClientError> {
    import_coin_index_signatures::<CliSocks5Client, _>(
        args.common_args,
        args.database_key.database_key.as_ref(),
    )
    .await?;
    println!("successfully imported coin index signatures!");
    Ok(())
}
