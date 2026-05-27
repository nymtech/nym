// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_coconut_dkg_common::types::EpochId;
use nym_coconut_dkg_common::verification_key::VerificationKeyShare;
use nym_crypto::asymmetric::ed25519;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::warn;

pub trait Verifiable {
    fn verify_signature(&self, pub_key: &ed25519::PublicKey) -> bool;
}

pub trait TimestampedResponse {
    fn timestamp(&self) -> OffsetDateTime;
}

pub trait LegacyChainResponse {
    fn chain_synced(&self, now: OffsetDateTime, stall_threshold: Duration) -> bool;
}

pub trait ChainResponse: Verifiable + TimestampedResponse {
    fn chain_synced(&self) -> bool;

    fn chain_available(
        &self,
        pub_key: &ed25519::PublicKey,
        now: OffsetDateTime,
        stale_response_threshold: Duration,
    ) -> bool {
        if !self.verify_signature(pub_key) {
            warn!("failed signature verification on chain status response");
            return false;
        }

        // we rely on information provided from the api itself AS LONG AS it's not too outdated
        if self.timestamp() + stale_response_threshold < now {
            warn!("chain status response is stale");
            return false;
        }
        self.chain_synced()
    }
}

pub trait LegacySignerResponse {
    fn signer_identity(&self) -> &str;

    fn signer_verification_key(&self) -> &Option<String>;

    fn unprovable_signing_available(
        &self,
        pub_key: &ed25519::PublicKey,
        expected_verification_key: Option<VerificationKeyShare>,
        share_verified: bool,
    ) -> bool {
        if self.signer_identity() != pub_key.to_base58_string() {
            warn!("mismatched identity key on the legacy response");
            return false;
        }

        // the contract share hasn't been verified yet, so we're probably in the middle of DKG
        // thus if there's a bit of desync in the state, it's fine
        if !share_verified {
            return true;
        }

        if self.signer_verification_key() != &expected_verification_key {
            warn!("mismatched [ecash] verification key on the legacy response");
            return false;
        }

        true
    }
}

pub trait SignerResponse: Verifiable + TimestampedResponse {
    fn has_signing_keys(&self) -> bool;

    fn signer_disabled(&self) -> bool;

    fn is_ecash_signer(&self) -> bool;

    fn dkg_ecash_epoch_id(&self) -> EpochId;

    fn provable_signing_available(
        &self,
        pub_key: &ed25519::PublicKey,
        dkg_epoch_id: EpochId,
        now: OffsetDateTime,
        stale_response_threshold: Duration,
    ) -> bool {
        if !self.verify_signature(pub_key) {
            warn!("failed signature verification on chain status response");
            return false;
        }

        // we rely on information provided from the api itself AS LONG AS it's not too outdated
        if self.timestamp() + stale_response_threshold < now {
            warn!("stale signer response");
            return false;
        }

        if !self.has_signing_keys() {
            warn!("missing signing keys");
            return false;
        }

        if self.signer_disabled() {
            warn!("signer functionalities are explicitly disabled");
            return false;
        }

        if !self.is_ecash_signer() {
            warn!("signer doesn't recognise it's a signer for this epoch");
            return false;
        }

        if dkg_epoch_id != self.dkg_ecash_epoch_id() {
            warn!(
                "mismatched dkg epoch id. current: {dkg_epoch_id}, signer's: {}",
                self.dkg_ecash_epoch_id()
            );
            return false;
        }

        true
    }
}
