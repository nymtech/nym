// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliAuthenticatorClient;
use nym_authenticator::error::AuthenticatorError;
use nym_client_core::cli_helpers::client_import_credential::{
    import_credential, CommonClientImportTicketBookArgs,
};

pub async fn execute(args: CommonClientImportTicketBookArgs) -> Result<(), AuthenticatorError> {
    import_credential::<CliAuthenticatorClient, _>(args).await?;
    println!("successfully imported credential!");
    Ok(())
}
