// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! HTTP transport for talking to a nym-api geolocation producer.
//!
//! **Not implemented yet.** The producer that serves these routes is a separate change, so
//! every method returns an error naming what is missing rather than a plausible empty answer -
//! an empty record set is indistinguishable from a verified one with no entries, so returning
//! nothing would look like success.
//!
//! The shape is fixed by this change's spec even though the server side is not built: the
//! geolocation contract is served under its own route tree mirroring the directory's
//! (`/snapshot/latest`, `/snapshot/{height}`, `/{height}/records`) rather than by
//! reparameterising the directory routes into `/attested/{domain}/...`. The directory routes
//! are deployed, the handlers are not shared anyway since the record types differ, and a
//! concrete path keeps each tree's OpenAPI schema specific.
//!
//! One producer requirement rides on this and is not discoverable from either tree alone:
//! whichever nym-api serves both contracts must use the same snapshot cadence and retained
//! window for both, so a single height serves a consumer joining the two.

use async_trait::async_trait;
use nym_contract_attestation::{
    AttestationSource, AttestationSourceError, SignedDigestSnapshot, SnapshotData,
};
use nym_crypto::asymmetric::ed25519;
use nym_geolocation_contract_common::GeolocationRecord;
use nym_validator_client::nyxd::Height;

/// The geolocation record set at a height, as a producer would serve it.
pub type GeolocationSnapshotData = SnapshotData<GeolocationRecord>;

/// A geolocation attestation source backed by a nym-api client `C`.
///
/// Holds the client rather than a URL, matching the directory's own source: the
/// domain-fronting client rotates its endpoint internally, so a captured URL goes stale.
/// `identity` is the signer key expected from this producer, so a consumer can tell which
/// source produced an attestation without a network call.
pub struct NymApiGeolocationSource<C> {
    client: C,
    identity: ed25519::PublicKey,
}

impl<C> NymApiGeolocationSource<C> {
    pub fn new(client: C, identity: ed25519::PublicKey) -> Self {
        NymApiGeolocationSource { client, identity }
    }

    pub fn client(&self) -> &C {
        &self.client
    }
}

/// The same trait the directory's source implements. Nothing about it is directory- or
/// geolocation-shaped: a source instance is scoped to one contract by the routes it was built
/// against, and declares the record type that follows from it.
#[async_trait]
impl<C> AttestationSource for NymApiGeolocationSource<C>
where
    C: Send + Sync,
{
    /// This source is pointed at the geolocation route tree, so geolocation records are the
    /// only thing it can serve.
    type Record = GeolocationRecord;

    fn identity(&self) -> ed25519::PublicKey {
        self.identity
    }

    async fn latest_snapshot(&self) -> Result<SignedDigestSnapshot, AttestationSourceError> {
        Err(not_implemented("geolocation latest-snapshot retrieval"))
    }

    async fn snapshot_at(
        &self,
        _height: Height,
    ) -> Result<SignedDigestSnapshot, AttestationSourceError> {
        Err(not_implemented("geolocation snapshot-at-height retrieval"))
    }

    async fn snapshot_data(
        &self,
        _height: Height,
    ) -> Result<SnapshotData<Self::Record>, AttestationSourceError> {
        Err(not_implemented("geolocation snapshot-data retrieval"))
    }
}

/// The anchor treats a failing source as a non-answer, so an unserved route has to reach it as
/// a transport failure. The message names what is missing so it is not mistaken for a network
/// fault by whoever wires this up first.
fn not_implemented(what: &'static str) -> AttestationSourceError {
    AttestationSourceError::Transport(format!(
        "{what} is not implemented: the nym-api geolocation producer is a separate change"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_test_utils::helpers::dummy_ed25519_keypair;

    fn source() -> NymApiGeolocationSource<()> {
        NymApiGeolocationSource::new((), *dummy_ed25519_keypair(1).public_key())
    }

    /// The stubs fail loudly. A caller wiring the producer up before it exists gets an error
    /// naming what is missing, rather than an empty set that looks verified.
    #[tokio::test]
    async fn every_transport_method_reports_itself_unimplemented() {
        let source = source();

        let errors = [
            source.latest_snapshot().await.err(),
            source.snapshot_at(Height::from(1u32)).await.err(),
            source.snapshot_data(Height::from(1u32)).await.err(),
        ];

        for error in errors {
            let message = error.expect("must not succeed").to_string();
            assert!(
                message.contains("not implemented"),
                "error should name the gap, got: {message}"
            );
        }
    }

    /// The identity is real even while the routes are not: it is how a consumer tells which
    /// producer signed an attestation, and it costs no network call.
    #[test]
    fn the_expected_signer_identity_is_retained() {
        let kp = dummy_ed25519_keypair(1);
        let source = NymApiGeolocationSource::new((), *kp.public_key());

        assert_eq!(source.identity(), *kp.public_key());
    }
}
