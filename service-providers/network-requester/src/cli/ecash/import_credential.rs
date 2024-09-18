// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliNetworkRequesterClient;
use crate::error::NetworkRequesterError;
use nym_client_core::cli_helpers::client_import_credential::{
    import_credential, CommonClientImportTicketBookArgs,
};

pub async fn execute(args: CommonClientImportTicketBookArgs) -> Result<(), NetworkRequesterError> {
    import_credential::<CliNetworkRequesterClient, _>(args).await?;
    println!("successfully imported credential!");
    Ok(())
}
