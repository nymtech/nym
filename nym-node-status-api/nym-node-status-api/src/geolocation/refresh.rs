// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Reading the geolocation contract, verified, into a snapshot.

use crate::geolocation::{GeoSnapshot, GeoSnapshotHandle};
use anyhow::{Context, bail};
use nym_contract_anchor::anchor::proven::ProvenTrustAnchor;
use nym_geolocation_client::client::GeolocationClient;
use nym_geolocation_client::key::digest_item_key;
use nym_geolocation_client::policy::DefaultResolutionPolicy;
use nym_geolocation_client::verified::{DecodedLocation, SubjectEntries, VerifiedGeolocation};
use nym_geolocation_contract_common::payload;
use nym_geolocation_contract_common::payload::PayloadError;
use nym_validator_client::QueryHttpRpcNyxdClient;
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::Height;
use nym_validator_client::nyxd::contract_traits::NymContractsProvider;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// The verifying reader, and the snapshot it publishes into.
pub(crate) struct GeolocationRefresher<C> {
    geolocation: GeolocationClient<ProvenTrustAnchor<C>, C>,
    snapshot: GeoSnapshotHandle,
}

impl GeolocationRefresher<QueryHttpRpcNyxdClient> {
    /// The anchor and the reader each take a client by value, so each gets its own from
    /// `clone_query_client`. That clones the `NyxdClient` wrapper around a `reqwest::Client`,
    /// which is internally reference-counted, so the three share one connection pool rather
    /// than dialing the RPC three times.
    pub(crate) fn new(
        nyx_client: &QueryHttpRpcNyxdClient,
        snapshot: GeoSnapshotHandle,
    ) -> anyhow::Result<Self> {
        let contract = nyx_client
            .geolocation_contract_address()
            .context("the network configuration carries no geolocation contract address")?
            .clone();

        // the anchor prefixes the contract onto the item key itself, so the proven key is
        // reconstructed locally rather than taken from whatever an RPC says it is
        let anchor =
            ProvenTrustAnchor::new(nyx_client.clone_query_client(), contract, digest_item_key());

        Ok(Self {
            geolocation: GeolocationClient::new(anchor, nyx_client.clone_query_client()),
            snapshot,
        })
    }

    /// One verified read at `height`, published whole.
    ///
    /// The height is an argument rather than something this selects for itself, because the
    /// cadence grid is shared across every attested contract: when the directory read lands,
    /// one selection should drive both reads at the same height, which is the whole reason for
    /// reading on the grid at all. Selecting it in here would make that two selections that
    /// have to be reconciled.
    ///
    /// Publishing is the last thing this does, so every failure above it leaves the held
    /// snapshot exactly where it was.
    pub(crate) async fn refresh(&self, height: Height) -> anyhow::Result<()> {
        let verified = self
            .geolocation
            .verified_geolocation(height)
            .await
            .with_context(|| format!("failed to verify geolocation records at height {height}"))?;

        let locations = verified
            .resolve_all(&DefaultResolutionPolicy)
            .iter()
            .filter_map(|(node_id, entry)| {
                entry
                    .decoded()
                    .location()
                    .map(|loc| (*node_id, loc.clone()))
            })
            .collect::<HashMap<_, _>>();

        log_unusable(&verified, &locations);

        publish(&self.snapshot, height, locations, verified.len())
    }
}

/// Replace the held snapshot, unless doing so would discard locations.
///
/// `subjects` is only for the logs: how many the contract held at that height, against how many
/// of them this build could use.
fn publish(
    snapshot: &GeoSnapshotHandle,
    height: Height,
    locations: HashMap<NodeId, payload::Location>,
    subjects: usize,
) -> anyhow::Result<()> {
    let held = snapshot.load();

    if locations.is_empty() && !held.locations.is_empty() {
        bail!(
            "refusing to replace {} held locations with an empty snapshot at height {height}: \
             the contract held {subjects} subjects there and none resolved to a usable location",
            held.locations.len()
        );
    }

    if locations.is_empty() {
        warn!(
            "the geolocation contract holds no usable location at height {height}, so the dVPN \
             directory will be empty. Expected on a network the geolocator has not populated \
             yet, and a fault anywhere else"
        );
    } else {
        info!(
            "geolocation refreshed at height {height}: {} of {subjects} subjects resolved to a \
             location",
            locations.len()
        );
    }

    snapshot.store(GeoSnapshot { height, locations });

    Ok(())
}

/// Report every subject the contract holds entries for that this build could not use, at a
/// severity that says which of them is an alarm.
///
/// A node with no entry at all is not visible here, because it is simply absent from the
/// contract's set: that case is reported where the node ids are known, at the consumer's lookup.
fn log_unusable(verified: &VerifiedGeolocation, locations: &HashMap<NodeId, payload::Location>) {
    for (node_id, entries) in &verified.subjects {
        if locations.contains_key(node_id) {
            continue;
        }

        match unusable_reason(entries) {
            Some(Unusable::Malformed(err)) => error!(
                "node {node_id} carries a malformed geolocation payload: {err}. The contract \
                 checks a payload's size but never its content, so nothing on the write path \
                 would have rejected this"
            ),
            Some(Unusable::UnsupportedVersion(version)) => warn!(
                "node {node_id} carries a geolocation payload of version {version}, which this \
                 build has no decoder for: a payload version has been rolled out ahead of it"
            ),
            None => debug!(
                "node {node_id} has geolocation entries, all of them readable, none of which \
                 the resolution policy was willing to act on"
            ),
        }
    }
}

/// Why this build could not read any of one subject's entries.
enum Unusable<'a> {
    /// A version this build has no decoder for. Expected rather than wrong, and the early
    /// signal that a payload version has been rolled out ahead of this build.
    UnsupportedVersion(u8),

    /// A version this build understands whose content did not parse. Anomalous.
    Malformed(&'a PayloadError),
}

/// The most severe reason none of **this** subject's entries could be read, across its own
/// slots: a node can hold several measurements, a self-declaration and an override, and they
/// can fail differently.
///
/// `None` when every entry decoded and the policy declined them for some other reason, such as
/// a self-declaration whose attestation does not verify. Severity order rather than
/// first-found, so a malformed payload is not hidden behind an unreadable version that happens
/// to sit in an earlier slot.
fn unusable_reason(entries: &SubjectEntries) -> Option<Unusable<'_>> {
    let mut unsupported = None;

    for decoded in entries
        .measured
        .iter()
        .map(|entry| &entry.decoded)
        .chain(entries.self_declared.iter().map(|entry| &entry.decoded))
        .chain(entries.overridden.iter().map(|entry| &entry.decoded))
    {
        match decoded {
            DecodedLocation::Malformed(err) => return Some(Unusable::Malformed(err)),
            DecodedLocation::UnsupportedVersion(version) => {
                unsupported = Some(Unusable::UnsupportedVersion(*version))
            }
            DecodedLocation::Decoded(..) => {}
        }
    }

    unsupported
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_geolocation_client::verified::{OverrideEntry, SelfDeclaredEntry};
    use nym_geolocation_contract_common::LocationPayload;
    use time::OffsetDateTime;

    fn location(country: &str) -> payload::Location {
        payload::Location {
            two_letter_iso_country_code: country.to_string(),
            coordinates: None,
            city: String::new(),
            region: String::new(),
            org: String::new(),
            postal: String::new(),
            timezone: String::new(),
            asn: None,
        }
    }

    fn snapshot_of(height: u64, nodes: &[NodeId]) -> GeoSnapshot {
        GeoSnapshot {
            height: Height::try_from(height).expect("test height"),
            locations: nodes.iter().map(|id| (*id, location("CH"))).collect(),
        }
    }

    /// The payload bytes are irrelevant to everything under test: `decoded` is what this build
    /// made of them, and it is carried alongside rather than derived on the way through.
    fn payload_bytes() -> LocationPayload {
        LocationPayload::new_v1(&location("CH")).expect("a location encodes")
    }

    fn slot(decoded: DecodedLocation) -> OverrideEntry {
        OverrideEntry {
            checked_at: OffsetDateTime::UNIX_EPOCH,
            location: payload_bytes(),
            decoded,
        }
    }

    fn declared(decoded: DecodedLocation) -> SelfDeclaredEntry {
        SelfDeclaredEntry {
            checked_at: OffsetDateTime::UNIX_EPOCH,
            location: payload_bytes(),
            decoded,
            attestation: None,
        }
    }

    /// The rule the dVPN directory's existence rests on: an empty read is refused rather than
    /// published, because an empty country code drops every gateway at once.
    #[test]
    fn an_empty_result_does_not_replace_held_locations() {
        let handle = GeoSnapshotHandle::new();
        handle.store(snapshot_of(1_000, &[1, 2, 3]));

        let refused = publish(
            &handle,
            Height::try_from(1_100u64).unwrap(),
            HashMap::new(),
            0,
        );

        assert!(refused.is_err(), "an empty result replaced held locations");

        let held = handle.load();
        assert_eq!(held.locations.len(), 3, "held locations were discarded");
        assert_eq!(
            held.height.value(),
            1_000,
            "the held snapshot moved to the refused height"
        );
    }

    /// The other half of that rule, and the reason it is about replacement rather than about
    /// emptiness: a network whose geolocator has written nothing is not a fault, and reporting
    /// one every refresh would say nothing true.
    #[test]
    fn an_empty_result_replaces_an_empty_snapshot() {
        let handle = GeoSnapshotHandle::new();

        publish(
            &handle,
            Height::try_from(1_100u64).unwrap(),
            HashMap::new(),
            0,
        )
        .expect("an empty result over an empty snapshot is not a failure");

        assert_eq!(
            handle.load().height.value(),
            1_100,
            "the read height was not recorded"
        );
    }

    #[test]
    fn a_non_empty_result_replaces_whatever_is_held() {
        let handle = GeoSnapshotHandle::new();
        handle.store(snapshot_of(1_000, &[1, 2, 3]));

        publish(
            &handle,
            Height::try_from(1_100u64).unwrap(),
            snapshot_of(1_100, &[7]).locations,
            1,
        )
        .expect("a usable result publishes");

        let held = handle.load();
        assert_eq!(held.height.value(), 1_100);
        assert!(
            held.locations.contains_key(&7),
            "the new set was not published"
        );
        assert_eq!(
            held.locations.len(),
            1,
            "the old set was merged rather than replaced"
        );
    }

    /// Severity order across one subject's slots. The malformed payload sits in a later slot
    /// than the unreadable version deliberately: first-found would report the wrong one, and
    /// the malformed one is the anomaly worth waking up for.
    #[test]
    fn a_malformed_payload_outranks_an_unreadable_version() {
        let entries = SubjectEntries {
            measured: Vec::new(),
            self_declared: Some(declared(DecodedLocation::UnsupportedVersion(2))),
            overridden: Some(slot(DecodedLocation::Malformed(PayloadError::Malformed(
                "content".to_string(),
            )))),
        };

        assert!(matches!(
            unusable_reason(&entries),
            Some(Unusable::Malformed(..))
        ));
    }

    #[test]
    fn an_unreadable_version_is_reported_when_nothing_is_worse() {
        let entries = SubjectEntries {
            measured: Vec::new(),
            self_declared: Some(declared(DecodedLocation::Decoded(Box::new(location("CH"))))),
            overridden: Some(slot(DecodedLocation::UnsupportedVersion(2))),
        };

        assert!(matches!(
            unusable_reason(&entries),
            Some(Unusable::UnsupportedVersion(2))
        ));
    }

    /// A subject whose entries all read is not unusable, whatever the policy then made of them:
    /// that distinction is what keeps a declined self-declaration out of the warning logs.
    #[test]
    fn readable_entries_are_not_a_reason() {
        let entries = SubjectEntries {
            measured: Vec::new(),
            self_declared: Some(declared(DecodedLocation::Decoded(Box::new(location("CH"))))),
            overridden: None,
        };

        assert!(unusable_reason(&entries).is_none());
    }
}
