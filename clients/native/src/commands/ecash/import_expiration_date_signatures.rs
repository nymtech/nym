// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::CliNativeClient;
use crate::error::ClientError;
use nym_client_core::cli_helpers::client_import_expiration_date_signatures::{
    import_expiration_date_signatures, CommonClientImportExpirationDateSignaturesArgs,
};
use nym_client_core::cli_helpers::database_key::DatabaseKeyArgs;

#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    common_args: CommonClientImportExpirationDateSignaturesArgs,

    #[command(flatten)]
    database_key: DatabaseKeyArgs,
}

pub(crate) async fn execute(args: Args) -> Result<(), ClientError> {
    import_expiration_date_signatures::<CliNativeClient, _>(
        args.common_args,
        args.database_key.database_key.as_ref(),
    )
    .await?;
    println!("successfully imported expiration date signatures!");
    Ok(())
}
