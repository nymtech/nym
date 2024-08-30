// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliNetworkRequesterClient;
use crate::error::NetworkRequesterError;
use nym_client_core::cli_helpers::client_import_coin_index_signatures::{
    import_coin_index_signatures, CommonClientImportCoinIndexSignaturesArgs,
};

pub(crate) async fn execute(
    args: CommonClientImportCoinIndexSignaturesArgs,
) -> Result<(), NetworkRequesterError> {
    import_coin_index_signatures::<CliNetworkRequesterClient, _>(args).await?;
    println!("successfully imported coin index signatures!");
    Ok(())
}
