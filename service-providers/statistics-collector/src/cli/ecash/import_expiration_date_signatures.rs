// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliStatsCollectorClient;
use nym_statistics_collector::error::StatsCollectorError;
use nym_client_core::cli_helpers::client_import_expiration_date_signatures::{
    import_expiration_date_signatures, CommonClientImportExpirationDateSignaturesArgs,
};

pub(crate) async fn execute(
    args: CommonClientImportExpirationDateSignaturesArgs,
) -> Result<(), StatsCollectorError> {
    import_expiration_date_signatures::<CliStatsCollectorClient, _>(args).await?;
    println!("successfully imported expiration date signatures!");
    Ok(())
}
