// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealer_information::RawDealerInformation;
use crate::helper_traits::{
    ChainResponse, LegacyChainResponse, LegacySignerResponse, SignerResponse,
};
use nym_coconut_dkg_common::types::EpochId;
use nym_coconut_dkg_common::verification_key::VerificationKeyShare;
use nym_crypto::asymmetric::ed25519;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::OffsetDateTime;
use utoipa::ToSchema;

pub(crate) const CHAIN_STALL_THRESHOLD: Duration = Duration::from_secs(5 * 60);
pub(crate) const STALE_RESPONSE_THRESHOLD: Duration = Duration::from_secs(5 * 60);

// the reason for generics is not to remove duplication of code,
// but because without them, we'd be having problems with circular dependencies,
// i.e. nym-api-requests depending on ecash-signer-check-types and
// ecash-signer-check-types needing nym-api-requests
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum Status<L, T> {
    /// The API, even though it reports correct version, did not response to the status query
    Unreachable,

    /// The API is running an outdated version that does not expose the required endpoint
    Outdated,

    /// Response to the legacy (unsigned) status query
    ReachableLegacy { response: Box<L> },

    /// Response to the current (signed) status query
    Reachable { response: Box<T> },
}

impl<L, T> Status<L, T>
where
    L: LegacyChainResponse,
    T: ChainResponse,
{
    pub fn chain_available(&self, pub_key: ed25519::PublicKey) -> bool {
        let now = OffsetDateTime::now_utc();

        match self {
            Status::Unreachable | Status::Outdated => false,
            Status::ReachableLegacy { response } => {
                response.chain_synced(now, CHAIN_STALL_THRESHOLD)
            }
            Status::Reachable { response } => {
                response.chain_available(&pub_key, now, STALE_RESPONSE_THRESHOLD)
            }
        }
    }

    pub fn chain_provably_stalled(&self, pub_key: ed25519::PublicKey) -> bool {
        let now = OffsetDateTime::now_utc();

        match self {
            Status::Unreachable | Status::Outdated | Status::ReachableLegacy { .. } => false,
            Status::Reachable { response } => {
                !response.chain_available(&pub_key, now, STALE_RESPONSE_THRESHOLD)
            }
        }
    }

    pub fn chain_unprovably_stalled(&self) -> bool {
        let now = OffsetDateTime::now_utc();

        match self {
            Status::Unreachable | Status::Outdated | Status::Reachable { .. } => false,
            Status::ReachableLegacy { response } => {
                !response.chain_synced(now, CHAIN_STALL_THRESHOLD)
            }
        }
    }
}

impl<L, T> Status<L, T>
where
    L: LegacySignerResponse,
    T: SignerResponse,
{
    pub fn signing_available(
        &self,
        pub_key: ed25519::PublicKey,
        dkg_epoch_id: u64,
        expected_verification_key: Option<VerificationKeyShare>,
        share_verified: bool,
    ) -> bool {
        let now = OffsetDateTime::now_utc();

        match self {
            Status::Unreachable | Status::Outdated => false,
            Status::ReachableLegacy { response } => response.unprovable_signing_available(
                &pub_key,
                expected_verification_key,
                share_verified,
            ),
            Status::Reachable { response } => response.provable_signing_available(
                &pub_key,
                dkg_epoch_id,
                now,
                STALE_RESPONSE_THRESHOLD,
            ),
        }
    }

    pub fn signing_provably_unavailable(
        &self,
        pub_key: ed25519::PublicKey,
        dkg_epoch_id: EpochId,
    ) -> bool {
        let now = OffsetDateTime::now_utc();

        match self {
            Status::Unreachable | Status::Outdated | Status::ReachableLegacy { .. } => false,
            Status::Reachable { response } => !response.provable_signing_available(
                &pub_key,
                dkg_epoch_id,
                now,
                STALE_RESPONSE_THRESHOLD,
            ),
        }
    }

    pub fn signing_unprovably_unavailable(
        &self,
        pub_key: ed25519::PublicKey,
        expected_verification_key: Option<VerificationKeyShare>,
        share_verified: bool,
    ) -> bool {
        match self {
            Status::Unreachable | Status::Outdated | Status::Reachable { .. } => false,
            Status::ReachableLegacy { response } => !response.unprovable_signing_available(
                &pub_key,
                expected_verification_key,
                share_verified,
            ),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SignerResult<LS, TS, LC, TC> {
    pub dkg_epoch_id: u64,
    pub information: RawDealerInformation,
    pub status: SignerStatus<LS, TS, LC, TC>,
}

impl<LS, TS, LC, TC> SignerResult<LS, TS, LC, TC> {
    pub fn signer_unreachable(&self) -> bool {
        matches!(self.status, SignerStatus::Unreachable)
    }

    pub fn malformed_details(&self) -> bool {
        self.information.parse().is_err()
    }
}

impl<LS, TS, LC, TC> SignerResult<LS, TS, LC, TC>
where
    LC: LegacyChainResponse,
    TC: ChainResponse,
{
    pub fn unknown_chain_status(&self) -> bool {
        let Ok(_) = self.information.parse() else {
            return true;
        };
        if let SignerStatus::Tested { .. } = &self.status {
            return false;
        }
        true
    }

    pub fn chain_available(&self) -> bool {
        let Ok(parsed_info) = self.information.parse() else {
            return false;
        };

        let SignerStatus::Tested { result } = &self.status else {
            return false;
        };
        result
            .local_chain_status
            .chain_available(parsed_info.public_key)
    }

    pub fn chain_provably_stalled(&self) -> bool {
        let Ok(parsed_info) = self.information.parse() else {
            return false;
        };

        let SignerStatus::Tested { result } = &self.status else {
            return false;
        };

        result
            .local_chain_status
            .chain_provably_stalled(parsed_info.public_key)
    }

    pub fn chain_unprovably_stalled(&self) -> bool {
        let SignerStatus::Tested { result } = &self.status else {
            return false;
        };

        result.local_chain_status.chain_unprovably_stalled()
    }
}

impl<LS, TS, LC, TC> SignerResult<LS, TS, LC, TC>
where
    LS: LegacySignerResponse,
    TS: SignerResponse,
{
    pub fn unknown_signing_status(&self) -> bool {
        let Ok(_) = self.information.parse() else {
            return true;
        };
        if let SignerStatus::Tested { .. } = &self.status {
            return false;
        }
        true
    }

    pub fn signing_available(&self) -> bool {
        let Ok(parsed_info) = self.information.parse() else {
            return false;
        };

        let SignerStatus::Tested { result } = &self.status else {
            return false;
        };
        result.signing_status.signing_available(
            parsed_info.public_key,
            self.dkg_epoch_id,
            parsed_info.verification_key_share,
            parsed_info.share_verified,
        )
    }

    pub fn signing_provably_unavailable(&self) -> bool {
        let Ok(parsed_info) = self.information.parse() else {
            return false;
        };

        let SignerStatus::Tested { result } = &self.status else {
            return false;
        };

        result
            .signing_status
            .signing_provably_unavailable(parsed_info.public_key, self.dkg_epoch_id)
    }

    pub fn signing_unprovably_unavailable(&self) -> bool {
        let Ok(parsed_info) = self.information.parse() else {
            return false;
        };

        let SignerStatus::Tested { result } = &self.status else {
            return false;
        };

        result.signing_status.signing_unprovably_unavailable(
            parsed_info.public_key,
            parsed_info.verification_key_share,
            parsed_info.share_verified,
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum SignerStatus<LS, TS, LC, TC> {
    Unreachable,
    ProvidedInvalidDetails,
    Tested {
        result: SignerTestResult<LS, TS, LC, TC>,
    },
}

impl<LS, TS, LC, TC> SignerStatus<LS, TS, LC, TC> {
    pub fn with_details(
        self,
        information: impl Into<RawDealerInformation>,
        dkg_epoch_id: u64,
    ) -> SignerResult<LS, TS, LC, TC> {
        SignerResult {
            dkg_epoch_id,
            status: self,
            information: information.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SignerTestResult<LS, TS, LC, TC> {
    pub reported_version: String,
    pub signing_status: Status<LS, TS>,
    pub local_chain_status: Status<LC, TC>,
}
