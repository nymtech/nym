// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::status::STALE_RESPONSE_THRESHOLD;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::ecash::models::EcashSignerStatusResponse;
use nym_validator_client::models::SignerInformationResponse;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::VerificationKeyShare;
use time::OffsetDateTime;
use tracing::{debug, warn};

// Magura (possibly earlier)
pub(crate) const MINIMUM_LEGACY_VERSION: semver::Version = semver::Version::new(1, 1, 46);

// Emmental
pub(crate) const MINIMUM_VERSION: semver::Version = semver::Version::new(1, 1, 62);

#[derive(Debug)]
pub enum SigningStatus {
    /// The API, even though it reports correct version, did not response to the status query
    Unreachable,

    /// The API is running an outdated version that does not expose the required endpoint
    Outdated,

    /// Response to the legacy (unsigned) status query
    ReachableLegacy {
        response: Box<SignerInformationResponse>,
    },

    /// Response to the current (signed) status query
    Reachable { response: EcashSignerStatusResponse },
}

impl SigningStatus {
    pub fn available(
        &self,
        pub_key: ed25519::PublicKey,
        dkg_epoch_id: u64,
        expected_verification_key: Option<VerificationKeyShare>,
        share_verified: bool,
    ) -> bool {
        let now = OffsetDateTime::now_utc();
        match self {
            SigningStatus::Unreachable | SigningStatus::Outdated => false,
            SigningStatus::ReachableLegacy { response } => {
                if response.identity != pub_key.to_base58_string() {
                    warn!("mismatched identity key on the legacy response");
                    return false;
                }

                // the contract share hasn't been verified yet, so we're probably in the middle of DKG
                // thus if there's a bit of desync in the state, it's fine
                if !share_verified {
                    return true;
                }

                if response.verification_key != expected_verification_key {
                    warn!("mismatched [ecash] verification key on the legacy response");
                    return false;
                }

                true
            }
            SigningStatus::Reachable { response } => {
                if !response.verify_signature(&pub_key) {
                    warn!("failed signature verification on signer status response");
                    return false;
                }

                if response.body.current_time + STALE_RESPONSE_THRESHOLD < now {
                    return false;
                }

                if !response.body.has_signing_keys {
                    debug!("missing signing keys");
                    return false;
                }

                if response.body.signer_disabled {
                    debug!("signer functionalities explicitly disabled");
                    return false;
                }

                if !response.body.is_ecash_signer {
                    debug!("signer doesn't recognise it's a signer for this epoch");
                    return false;
                }

                if dkg_epoch_id != response.body.dkg_ecash_epoch_id {
                    debug!(
                        "mismatched dkg epoch id. current: {dkg_epoch_id}, signer's: {}",
                        response.body.dkg_ecash_epoch_id
                    );
                    return false;
                }

                true
            }
        }
    }
}
