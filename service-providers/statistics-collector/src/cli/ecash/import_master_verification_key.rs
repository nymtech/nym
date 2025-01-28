// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliStatsCollectorClient;
use nym_statistics_collector::error::StatsCollectorError;
use nym_client_core::cli_helpers::client_import_master_verification_key::{
    import_master_verification_key, CommonClientImportMasterVerificationKeyArgs,
};

pub(crate) async fn execute(
    args: CommonClientImportMasterVerificationKeyArgs,
) -> Result<(), StatsCollectorError> {
    import_master_verification_key::<CliStatsCollectorClient, _>(args).await?;
    println!("successfully imported master verification key!");
    Ok(())
}
