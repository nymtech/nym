// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::CliSocks5Client;
use crate::error::Socks5ClientError;
use nym_client_core::cli_helpers::client_import_credential::{
    import_credential, CommonClientImportTicketBookArgs,
};

pub async fn execute(args: CommonClientImportTicketBookArgs) -> Result<(), Socks5ClientError> {
    import_credential::<CliSocks5Client, _>(args).await?;
    println!("successfully imported credential!");
    Ok(())
}
