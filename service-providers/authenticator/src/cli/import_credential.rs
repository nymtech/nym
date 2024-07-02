// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliAuthenticatorClient;
use nym_authenticator::error::AuthenticatorError;
use nym_client_core::cli_helpers::client_import_credential::{
    import_credential, CommonClientImportCredentialArgs,
};

pub(crate) async fn execute(
    args: CommonClientImportCredentialArgs,
) -> Result<(), AuthenticatorError> {
    import_credential::<CliAuthenticatorClient, _>(args).await?;
    println!("successfully imported credential!");
    Ok(())
}
