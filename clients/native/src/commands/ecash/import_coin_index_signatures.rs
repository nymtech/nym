// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::CliNativeClient;
use crate::error::ClientError;
use nym_client_core::cli_helpers::client_import_coin_index_signatures::{
    import_coin_index_signatures, CommonClientImportCoinIndexSignaturesArgs,
};

pub(crate) async fn execute(
    args: CommonClientImportCoinIndexSignaturesArgs,
) -> Result<(), ClientError> {
    import_coin_index_signatures::<CliNativeClient, _>(args).await?;
    println!("successfully imported coin index signatures!");
    Ok(())
}
