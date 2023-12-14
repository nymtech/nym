// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymRewarderError;
use crate::rewarder::credential_issuance::types::CredentialIssuanceResults;
use crate::rewarder::epoch::Epoch;
use tracing::info;

pub mod types;

pub struct CredentialIssuance {}

impl CredentialIssuance {
    pub(crate) async fn get_signed_blocks_results(
        &self,
        current_epoch: Epoch,
    ) -> Result<CredentialIssuanceResults, NymRewarderError> {
        info!(
            "looking up credential issuers for epoch {} ({} - {})",
            current_epoch.id,
            current_epoch.start_rfc3339(),
            current_epoch.end_rfc3339()
        );

        Ok(CredentialIssuanceResults {})
    }
}
