// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::ed25519;
use nym_validator_client::ecash::models::EcashSignerStatusResponse;
use tracing::{debug, warn};

// Emmental
pub(crate) const MINIMUM_VERSION: semver::Version = semver::Version::new(1, 1, 62);

#[derive(Debug)]
pub enum SigningStatus {
    /// The API, even though it reports correct version, did not response to the status query
    Unreachable,

    /// The API is running an outdated version that does not expose the required endpoint
    Outdated,

    /// Response to the status query
    Reachable { response: EcashSignerStatusResponse },
}

impl SigningStatus {
    pub fn available(&self, pub_key: ed25519::PublicKey, dkg_epoch_id: u64) -> bool {
        let SigningStatus::Reachable { response } = self else {
            return false;
        };
        if !response.verify_signature(&pub_key) {
            warn!("failed signature verification on signer status response");
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
