// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::CliSocks5Client;
use crate::error::Socks5ClientError;
use nym_client_core::cli_helpers::client_import_expiration_date_signatures::{
    import_expiration_date_signatures, CommonClientImportExpirationDateSignaturesArgs,
};

pub(crate) async fn execute(
    args: CommonClientImportExpirationDateSignaturesArgs,
) -> Result<(), Socks5ClientError> {
    import_expiration_date_signatures::<CliSocks5Client, _>(args).await?;
    println!("successfully imported expiration date signatures!");
    Ok(())
}
