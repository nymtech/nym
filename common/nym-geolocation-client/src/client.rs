// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The geolocation retrieval client: fetch the whole record set at a single height, prove it
//! against a trust anchor, and attribute each entry to whoever is allowed to have written it.
//!
//! Every read is pinned to one height, and that is load-bearing rather than tidy. The digest
//! is proven at `H`; if the records were paged at the chain tip instead, a write landing
//! mid-enumeration would produce a mismatch indistinguishable from tampering. Node identities
//! are read at `H` too, so a node that bonded or unbonded afterwards cannot change whether an
//! older self-declaration attributes.

use crate::verified::VerifiedGeolocation;
use crate::verify::verify_records;
use nym_contract_anchor::anchor::TrustAnchor;
use nym_contract_anchor::error::VerificationError;
use nym_contract_anchor::whole_set::node_identities_at;
use nym_validator_client::nyxd::Height;
use nym_validator_client::nyxd::contract_traits::{
    PinnedGeolocationQueryClient, PinnedMixnetQueryClient,
};

/// A verifiable geolocation reader. Composes a trust anchor (which produces the digest to
/// trust at a height) with height-pinned chain queries; the anchor can be swapped - proven,
/// light-client or attested - without touching the verification core.
pub struct GeolocationClient<A, C> {
    anchor: A,
    client: C,
}

impl<A, C> GeolocationClient<A, C> {
    pub fn client(&self) -> &C {
        &self.client
    }
}

impl<A, C> GeolocationClient<A, C>
where
    A: TrustAnchor + Sync,
    C: PinnedGeolocationQueryClient + PinnedMixnetQueryClient + Sync,
{
    pub fn new(anchor: A, client: C) -> Self {
        GeolocationClient { anchor, client }
    }

    /// Retrieve and verify every geolocation record at `height`.
    ///
    /// Fails closed at every step: an anchor that cannot establish a digest, a record set that
    /// does not recompute to it, and a chain query that cannot be served all return an error
    /// with no records attached.
    pub async fn verified_geolocation(
        &self,
        height: Height,
    ) -> Result<VerifiedGeolocation, VerificationError> {
        let trusted = self.anchor.trusted_digest(height).await?;
        let records = self
            .client
            .get_all_geolocation_records_at_height(height)
            .await?;
        let identities = node_identities_at(&self.client, height).await?;

        verify_records(&trusted, records, &identities)
    }
}
