// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The performance retrieval client: read the whole committed set at a single height and
//! verify it against a trust anchor, with the bond set at that height alongside it.
//!
//! Every read is pinned to one height. The digest is proven at `H`; if the entries were paged
//! at the chain tip instead, a submission landing mid-enumeration would produce a mismatch
//! indistinguishable from tampering.

use crate::verify::VerifiedPerformance;
use nym_contract_anchor::anchor::TrustAnchor;
use nym_contract_anchor::error::VerificationError;
use nym_contract_anchor::whole_set::verify_chain_read;
use nym_validator_client::nyxd::Height;
use nym_validator_client::nyxd::contract_traits::{
    PinnedMixnetQueryClient, PinnedPerformanceQueryClient,
};

/// A verifiable performance reader. Composes a trust anchor (which produces the digest to
/// trust at a height) with height-pinned chain queries; the anchor can be swapped - proven,
/// light-client or attested - without touching the verification.
///
/// Its anchor must be built with the performance contract's address and the bare item key
/// [`storage_keys::DIGEST_STATE`](nym_performance_contract_common::constants::storage_keys::DIGEST_STATE)
/// (`.as_bytes()`), never the prefixed raw storage key: the anchor prefixes the contract
/// itself, so a pre-prefixed key never matches a proof.
pub struct PerformanceClient<A, C> {
    anchor: A,
    client: C,
}

impl<A, C> PerformanceClient<A, C> {
    pub fn new(anchor: A, client: C) -> Self {
        PerformanceClient { anchor, client }
    }

    pub fn client(&self) -> &C {
        &self.client
    }

    /// The anchor every read is verified against. A producer takes the `app_hash` it signs
    /// from here, so it comes from the anchor that established the digest.
    pub fn anchor(&self) -> &A {
        &self.anchor
    }
}

impl<A, C> PerformanceClient<A, C>
where
    A: TrustAnchor + Sync,
    C: PinnedPerformanceQueryClient + PinnedMixnetQueryClient + Sync,
{
    /// Retrieve and verify every committed entry at `height`, with the bond set at `height`.
    ///
    /// Fails closed at every step: an anchor that cannot establish a digest, an entry set that
    /// does not recompute to it, and a chain query that cannot be served all return an error
    /// with no entries attached.
    pub async fn verified_performance(
        &self,
        height: Height,
    ) -> Result<VerifiedPerformance, VerificationError> {
        let records = self.client.get_all_latest_performance_at_height(height);
        let verified = verify_chain_read(&self.anchor, &self.client, height, records).await?;
        Ok(verified.into())
    }
}
