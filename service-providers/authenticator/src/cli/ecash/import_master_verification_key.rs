// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliAuthenticatorClient;
use nym_authenticator::error::AuthenticatorError;
use nym_client_core::cli_helpers::client_import_master_verification_key::{
    import_master_verification_key, CommonClientImportMasterVerificationKeyArgs,
};

pub(crate) async fn execute(
    args: CommonClientImportMasterVerificationKeyArgs,
) -> Result<(), AuthenticatorError> {
    import_master_verification_key::<CliAuthenticatorClient, _>(args).await?;
    println!("successfully imported master verification key!");
    Ok(())
}
