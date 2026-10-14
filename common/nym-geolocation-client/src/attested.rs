// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Whole-set geolocation retrieval against an [`AttestedTrustAnchor`], with no chain
//! connection.
//!
//! The anchor checks a source's records and identity map against the quorum-agreed snapshot;
//! this module groups the verified records exactly as the RPC-backed read does.

use crate::error::GeolocationClientError;
use crate::verified::VerifiedGeolocation;
use async_trait::async_trait;
use nym_contract_anchor::anchor::attested::AttestedTrustAnchor;
use nym_contract_attestation::AttestationSource;
use nym_geolocation_contract_common::GeolocationRecord;
use nym_validator_client::nyxd::Height;

/// Geolocation reads over an attested anchor.
#[async_trait]
pub trait AttestedGeolocationExt {
    /// The whole geolocation record set at `height`, fetched from the anchor's sources and
    /// verified against the quorum-agreed snapshot, with each self-declaration attributed
    /// against the attested identity map.
    async fn verified_geolocation(
        &self,
        height: Height,
    ) -> Result<VerifiedGeolocation, GeolocationClientError>;
}

#[async_trait]
impl<S> AttestedGeolocationExt for AttestedTrustAnchor<S>
where
    S: AttestationSource<Record = GeolocationRecord> + Sync,
{
    async fn verified_geolocation(
        &self,
        height: Height,
    ) -> Result<VerifiedGeolocation, GeolocationClientError> {
        let verified = self.verified_snapshot_data(height).await?;

        Ok(VerifiedGeolocation::from_verified_records(
            height,
            verified.data.records,
            &verified.data.node_identities,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attestation::AttestationStatus;
    use crate::test_records::{measured, self_declared_signed, signing_keypair, whitelisted};
    use nym_contract_anchor::whole_set::recompute_accumulator;
    use nym_contract_attestation::source::mock::{
        MockAttestationSource, mock_app_hash, mock_chain_id, mock_contract,
    };
    use nym_contract_attestation::{DigestSnapshot, SnapshotData, node_identities_hash};
    use nym_crypto::asymmetric::ed25519;
    use std::collections::{BTreeMap, HashMap, HashSet};

    const HEIGHT: u32 = 100;

    /// A whitelisted agent's measurement and node 1's self-declaration, signed by `node`, with
    /// the identity map the self-declaration attributes against.
    fn consistent_data(node: &ed25519::KeyPair) -> SnapshotData<GeolocationRecord> {
        SnapshotData {
            height: Height::from(HEIGHT),
            records: vec![
                whitelisted("agent-one"),
                measured(2, "agent-one", 1_700_000_000, b"loc-two"),
                self_declared_signed(1, node, 1_700_000_050, b"sd"),
            ],
            node_identities: BTreeMap::from([(1, *node.public_key())]),
        }
    }

    fn tampered_data(node: &ed25519::KeyPair) -> SnapshotData<GeolocationRecord> {
        let mut data = consistent_data(node);
        data.records
            .push(measured(3, "agent-one", 1_700_000_200, b"rogue"));
        data
    }

    fn snapshot_of(data: &SnapshotData<GeolocationRecord>) -> DigestSnapshot {
        DigestSnapshot {
            chain_id: mock_chain_id(),
            contract: mock_contract(0),
            height: data.height,
            app_hash: mock_app_hash(1),
            accumulator: recompute_accumulator(&data.records),
            node_identities_hash: node_identities_hash(&data.node_identities),
        }
    }

    /// A source attesting `snapshot` and serving `data` at its height.
    fn source(
        signer: &ed25519::KeyPair,
        snapshot: &DigestSnapshot,
        data: SnapshotData<GeolocationRecord>,
    ) -> MockAttestationSource<GeolocationRecord> {
        let signed = snapshot.clone().signed(signer);
        MockAttestationSource::new(
            *signer.public_key(),
            signed.clone(),
            HashMap::from([(snapshot.height, signed)]),
        )
        .with_snapshot_data(snapshot.height, data)
    }

    fn anchor(
        sources: Vec<MockAttestationSource<GeolocationRecord>>,
        signers: [&ed25519::KeyPair; 2],
    ) -> AttestedTrustAnchor<MockAttestationSource<GeolocationRecord>> {
        let trusted = HashSet::from(signers.map(|kp| *kp.public_key()));
        AttestedTrustAnchor::new(sources, trusted, 2, mock_chain_id(), mock_contract(0)).unwrap()
    }

    #[tokio::test]
    async fn returns_the_verified_geolocation_with_attested_attribution() {
        let a = signing_keypair(1);
        let b = signing_keypair(2);
        let node = signing_keypair(10);
        let data = consistent_data(&node);
        let snapshot = snapshot_of(&data);

        let anchor = anchor(
            vec![
                source(&a, &snapshot, data.clone()),
                source(&b, &snapshot, data),
            ],
            [&a, &b],
        );

        let verified = anchor
            .verified_geolocation(Height::from(HEIGHT))
            .await
            .unwrap();
        assert_eq!(verified.height, Height::from(HEIGHT));
        assert_eq!(verified.len(), 2);

        // the identity map is the attested one, so the signature check means something
        let entry = verified
            .get_subject(1)
            .unwrap()
            .self_declared
            .as_ref()
            .unwrap();
        assert_eq!(
            entry.attestation.as_ref().map(|a| a.status),
            Some(AttestationStatus::Verified)
        );
    }

    #[tokio::test]
    async fn skips_a_source_serving_bad_records_for_one_serving_good_records() {
        let a = signing_keypair(1);
        let b = signing_keypair(2);
        let node = signing_keypair(10);
        let snapshot = snapshot_of(&consistent_data(&node));

        let bad = source(&a, &snapshot, tampered_data(&node));
        let anchor = anchor(
            vec![bad.clone(), source(&b, &snapshot, consistent_data(&node))],
            [&a, &b],
        );

        let verified = anchor
            .verified_geolocation(Height::from(HEIGHT))
            .await
            .unwrap();
        assert_eq!(verified.len(), 2);
        assert!(verified.get_subject(3).is_none());
        assert_eq!(bad.snapshot_data_calls(), vec![Height::from(HEIGHT)]);
    }

    #[tokio::test]
    async fn fails_closed_when_every_source_serves_tampered_records() {
        let a = signing_keypair(1);
        let b = signing_keypair(2);
        let node = signing_keypair(10);
        let snapshot = snapshot_of(&consistent_data(&node));

        let anchor = anchor(
            vec![
                source(&a, &snapshot, tampered_data(&node)),
                source(&b, &snapshot, tampered_data(&node)),
            ],
            [&a, &b],
        );

        let err = anchor
            .verified_geolocation(Height::from(HEIGHT))
            .await
            .unwrap_err();
        assert!(matches!(err, GeolocationClientError::DigestMismatch));
    }
}
