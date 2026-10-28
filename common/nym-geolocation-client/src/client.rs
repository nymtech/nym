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

use crate::error::GeolocationClientError;
use crate::verified::VerifiedGeolocation;
use crate::verify::verify_records;
use nym_contract_anchor::anchor::TrustAnchor;
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::nyxd::Height;
use nym_validator_client::nyxd::contract_traits::{
    NymContractsProvider, PinnedGeolocationQueryClient, PinnedMixnetQueryClient,
};
use std::collections::BTreeMap;
use tracing::error;

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
    C: NymContractsProvider + PinnedGeolocationQueryClient + PinnedMixnetQueryClient + Sync,
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
    ) -> Result<VerifiedGeolocation, GeolocationClientError> {
        // the query layer reports a missing address as a generic chain-query failure; check
        // first so this crate's typed variant is what a caller sees
        self.client
            .geolocation_contract_address()
            .ok_or(GeolocationClientError::UnavailableGeolocationContract)?;

        let trusted = self.anchor.trusted_digest(height).await?;
        let records = self
            .client
            .get_all_geolocation_records_at_height(height)
            .await?;
        let identities = self.node_identities_at(height).await?;

        verify_records(&trusted, records, &identities)
    }

    /// The `NodeId -> ed25519 identity` mapping from the mixnet bond set at `height`.
    ///
    /// Read at the same height as the records, so a self-declaration is attributed against
    /// the identity the subject held when the entry was committed rather than whatever it
    /// holds now. Mirrors the directory client's own bond read; the height-pinned pagination
    /// underneath is shared, only this parse is not.
    async fn node_identities_at(
        &self,
        height: Height,
    ) -> Result<BTreeMap<NodeId, ed25519::PublicKey>, GeolocationClientError> {
        // see `verified_geolocation`: keep this crate's typed variant for a missing address
        self.client
            .mixnet_contract_address()
            .ok_or(GeolocationClientError::UnavailableMixnetContract)?;

        let bonds = self.client.get_all_nymnode_bonds_at_height(height).await?;

        let mut identities = BTreeMap::new();
        for bond in bonds {
            let Ok(identity) = bond.identity().parse() else {
                // should be impossible: the mixnet contract verified signatures under this
                // key when the node bonded
                error!(
                    "failed to parse identity key of node {} ({}) as a valid ed25519 public key",
                    bond.node_id,
                    bond.identity()
                );
                continue;
            };
            identities.insert(bond.node_id, identity);
        }

        Ok(identities)
    }
}
