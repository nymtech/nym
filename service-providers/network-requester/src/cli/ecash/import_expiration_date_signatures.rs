// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliNetworkRequesterClient;
use crate::error::NetworkRequesterError;
use nym_client_core::cli_helpers::client_import_expiration_date_signatures::{
    import_expiration_date_signatures, CommonClientImportExpirationDateSignaturesArgs,
};

pub(crate) async fn execute(
    args: CommonClientImportExpirationDateSignaturesArgs,
) -> Result<(), NetworkRequesterError> {
    import_expiration_date_signatures::<CliNetworkRequesterClient, _>(args).await?;
    println!("successfully imported expiration date signatures!");
    Ok(())
}
