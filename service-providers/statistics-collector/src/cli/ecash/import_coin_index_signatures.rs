// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliStatsCollectorClient;
use nym_statistics_collector::error::StatsCollectorError;
use nym_client_core::cli_helpers::client_import_coin_index_signatures::{
    import_coin_index_signatures, CommonClientImportCoinIndexSignaturesArgs,
};

pub(crate) async fn execute(
    args: CommonClientImportCoinIndexSignaturesArgs,
) -> Result<(), StatsCollectorError> {
    import_coin_index_signatures::<CliStatsCollectorClient, _>(args).await?;
    println!("successfully imported coin index signatures!");
    Ok(())
}
