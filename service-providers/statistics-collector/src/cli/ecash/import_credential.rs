// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliStatsCollectorClient;
use nym_statistics_collector::error::StatsCollectorError;
use nym_client_core::cli_helpers::client_import_credential::{
    import_credential, CommonClientImportTicketBookArgs,
};

pub async fn execute(args: CommonClientImportTicketBookArgs) -> Result<(), StatsCollectorError> {
    import_credential::<CliStatsCollectorClient, _>(args).await?;
    println!("successfully imported credential!");
    Ok(())
}
