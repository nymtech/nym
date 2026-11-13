// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::CliNativeClient;
use crate::error::ClientError;
use nym_client_core::cli_helpers::client_import_credential::{
    import_credential, CommonClientImportTicketBookArgs,
};
use nym_client_core::cli_helpers::database_key::DatabaseKeyArgs;

#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    common_args: CommonClientImportTicketBookArgs,

    #[command(flatten)]
    database_key: DatabaseKeyArgs,
}

pub(crate) async fn execute(args: Args) -> Result<(), ClientError> {
    import_credential::<CliNativeClient, _>(
        args.common_args,
        args.database_key.database_key.as_ref(),
    )
    .await?;
    println!("successfully imported credential!");
    Ok(())
}
