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
use nym_contract_anchor::anchor::TrustAnchor;
use nym_contract_anchor::error::VerificationError;
use nym_contract_anchor::whole_set::{VerifiedRecords, verify_chain_read};
use nym_geolocation_contract_common::GeolocationRecord;
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

    /// The anchor every read is verified against. A producer takes the `app_hash` it signs
    /// from here, so it comes from the anchor that established the digest.
    pub fn anchor(&self) -> &A {
        &self.anchor
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

    /// Retrieve and verify every geolocation record at `height`, returned exactly as read,
    /// whitelisted agents included: what a producer re-serves.
    ///
    /// Fails closed at every step: an anchor that cannot establish a digest, a record set that
    /// does not recompute to it, and a chain query that cannot be served all return an error
    /// with no records attached.
    pub async fn verified_records(
        &self,
        height: Height,
    ) -> Result<VerifiedRecords<GeolocationRecord>, VerificationError> {
        let records = self.client.get_all_geolocation_records_at_height(height);
        verify_chain_read(&self.anchor, &self.client, height, records).await
    }

    /// [`Self::verified_records`], grouped per subject.
    pub async fn verified_geolocation(
        &self,
        height: Height,
    ) -> Result<VerifiedGeolocation, VerificationError> {
        self.verified_records(height).await.map(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_records::{measured, record_set, signing_keypair};
    use cosmrs::tendermint::AppHash;
    use nym_contract_anchor::anchor::{ChainAnchor, TrustedDigest};
    use nym_contract_anchor::error::AnchorError;
    use nym_contract_anchor::whole_set::recompute_accumulator;
    use nym_contract_attestation::source::mock::mock_app_hash;
    use nym_crypto::asymmetric::ed25519;
    use nym_lthash::LtHash16;
    use nym_validator_client::nyxd::error::NyxdError;
    use nym_validator_client::nyxd::nym_mixnet_contract_common::nym_node::NodeDetailsResponse;
    use nym_validator_client::nyxd::nym_mixnet_contract_common::{
        Addr, Coin, NodeId, NymNode, NymNodeBond,
    };
    use std::collections::BTreeMap;

    fn height() -> Height {
        Height::from(100u32)
    }

    fn identities() -> BTreeMap<NodeId, ed25519::PublicKey> {
        BTreeMap::from([(1, *signing_keypair(1).public_key())])
    }

    /// An anchor trusting `accumulator` at every height.
    struct FixedAnchor {
        accumulator: LtHash16,
    }

    #[async_trait::async_trait]
    impl ChainAnchor for FixedAnchor {
        async fn trusted_app_hash(&self, _: Height) -> Result<AppHash, AnchorError> {
            Ok(mock_app_hash(1))
        }
    }

    #[async_trait::async_trait]
    impl TrustAnchor for FixedAnchor {
        async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, AnchorError> {
            Ok(TrustedDigest {
                height,
                accumulator: self.accumulator.clone(),
            })
        }
    }

    /// A chain holding `records` in the geolocation contract and one bond per entry of
    /// `identities`, at every height.
    struct FixtureChain {
        records: Vec<GeolocationRecord>,
        identities: BTreeMap<NodeId, ed25519::PublicKey>,
    }

    #[async_trait::async_trait]
    impl PinnedGeolocationQueryClient for FixtureChain {
        async fn get_all_geolocation_records_at_height(
            &self,
            _: Height,
        ) -> Result<Vec<GeolocationRecord>, NyxdError> {
            Ok(self.records.clone())
        }
    }

    #[async_trait::async_trait]
    impl PinnedMixnetQueryClient for FixtureChain {
        async fn get_nymnode_details_at_height(
            &self,
            _: NodeId,
            _: Height,
        ) -> Result<NodeDetailsResponse, NyxdError> {
            // the whole-set read never asks for a single node
            Err(NyxdError::unavailable_contract_address("mixnet contract"))
        }

        async fn get_all_nymnode_bonds_at_height(
            &self,
            _: Height,
        ) -> Result<Vec<NymNodeBond>, NyxdError> {
            Ok(self
                .identities
                .iter()
                .map(|(node_id, identity)| {
                    let node = NymNode {
                        host: "1.2.3.4".to_string(),
                        custom_http_port: None,
                        identity_key: identity.to_base58_string(),
                    };
                    NymNodeBond::new(
                        *node_id,
                        Addr::unchecked("owner"),
                        Coin::new(1u128, "unym"),
                        node,
                        0,
                    )
                })
                .collect())
        }
    }

    /// A client whose chain serves `served`, under an anchor trusting the digest `committed`
    /// recomputes to.
    fn client_serving(
        committed: &[GeolocationRecord],
        served: Vec<GeolocationRecord>,
    ) -> GeolocationClient<FixedAnchor, FixtureChain> {
        GeolocationClient::new(
            FixedAnchor {
                accumulator: recompute_accumulator(committed),
            },
            FixtureChain {
                records: served,
                identities: identities(),
            },
        )
    }

    #[tokio::test]
    async fn the_grouped_read_is_the_grouping_of_the_raw_read() {
        let records = record_set();
        let client = client_serving(&records, records.clone());

        let raw = client.verified_records(height()).await.unwrap();
        assert_eq!(raw.height, height());
        assert_eq!(raw.records, records);
        // the whitelist is part of what a producer re-serves, not filtered out of it
        assert!(
            raw.records
                .iter()
                .any(|record| matches!(record, GeolocationRecord::WhitelistedAgent(_)))
        );
        assert_eq!(raw.accumulator, recompute_accumulator(&records));
        assert_eq!(raw.node_identities, identities());

        let grouped = client.verified_geolocation(height()).await.unwrap();
        assert_eq!(grouped, VerifiedGeolocation::from(raw));
    }

    #[tokio::test]
    async fn a_set_that_does_not_recompute_fails_both_reads() {
        let records = record_set();
        let mut tampered = records.clone();
        tampered.push(measured(9, "agent-one", 1_700_000_900, b"rogue"));
        let client = client_serving(&records, tampered);

        assert!(matches!(
            client.verified_records(height()).await,
            Err(VerificationError::DigestMismatch)
        ));
        assert!(matches!(
            client.verified_geolocation(height()).await,
            Err(VerificationError::DigestMismatch)
        ));
    }
}
