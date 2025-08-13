// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::chain_status::LocalChainStatus;
use crate::dealer_information::RawDealerInformation;
use crate::signing_status::SigningStatus;
use std::time::Duration;

pub(crate) const STALE_RESPONSE_THRESHOLD: Duration = Duration::from_secs(5 * 60);

#[derive(Debug)]
pub struct SignerResult {
    pub dkg_epoch_id: u64,
    pub information: RawDealerInformation,
    pub status: SignerStatus,
}

impl SignerResult {
    pub fn chain_available(&self) -> bool {
        let Ok(parsed_info) = self.information.parse() else {
            return false;
        };

        let SignerStatus::Tested { result } = &self.status else {
            return false;
        };
        result.local_chain_status.available(parsed_info.public_key)
    }

    pub fn signer_available(&self) -> bool {
        let Ok(parsed_info) = self.information.parse() else {
            return false;
        };
        let SignerStatus::Tested { result } = &self.status else {
            return false;
        };

        result.signing_status.available(
            parsed_info.public_key,
            self.dkg_epoch_id,
            parsed_info.verification_key_share,
            parsed_info.share_verified,
        )
    }
}

#[derive(Debug)]
pub enum SignerStatus {
    Unreachable,
    ProvidedInvalidDetails,
    Tested { result: SignerTestResult },
}

impl SignerStatus {
    pub fn with_details(
        self,
        information: impl Into<RawDealerInformation>,
        dkg_epoch_id: u64,
    ) -> SignerResult {
        SignerResult {
            dkg_epoch_id,
            status: self,
            information: information.into(),
        }
    }
}

#[derive(Debug)]
pub struct SignerTestResult {
    pub reported_version: String,
    pub signing_status: SigningStatus,
    pub local_chain_status: LocalChainStatus,
}
