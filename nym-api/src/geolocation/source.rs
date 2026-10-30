// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::attested_snapshots::source::{AttestedSnapshotSource, VerifiedSet};
use crate::support::chain_anchor::{SharedChainAnchor, SharedContractAnchor};
use anyhow::Context;
use async_trait::async_trait;
use nym_contract_anchor::error::VerificationError;
use nym_geolocation_client::client::GeolocationClient;
use nym_geolocation_client::key::digest_item_key;
use nym_geolocation_contract_common::GeolocationRecord;
use nym_validator_client::nyxd::contract_traits::NymContractsProvider;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::QueryHttpRpcNyxdClient;
use tendermint::block::Height;

/// The geolocation contract's whole committed set at a height, whitelisted agents included,
/// verified through `GeolocationClient`.
pub(crate) struct GeolocationSnapshotSource {
    contract_address: AccountId,

    geolocation_client: GeolocationClient<SharedContractAnchor, QueryHttpRpcNyxdClient>,
}

impl GeolocationSnapshotSource {
    pub(crate) fn new(chain_anchor: &SharedChainAnchor) -> anyhow::Result<Self> {
        let query_client = chain_anchor.query_client().clone_query_client();
        let contract_address = query_client
            .geolocation_contract_address()
            .context("failed to get geolocation contract address")?
            .clone();
        let anchor = chain_anchor.contract_anchor(contract_address.clone(), digest_item_key());

        Ok(GeolocationSnapshotSource {
            contract_address,
            geolocation_client: GeolocationClient::new(anchor, query_client),
        })
    }
}

#[async_trait]
impl AttestedSnapshotSource for GeolocationSnapshotSource {
    type Record = GeolocationRecord;
    type Auxiliary = ();

    const NAME: &'static str = "geolocation";

    fn contract_address(&self) -> &AccountId {
        &self.contract_address
    }

    async fn verified_set(
        &self,
        height: Height,
        _previous: Option<Self::Auxiliary>,
    ) -> Result<VerifiedSet<Self>, VerificationError> {
        let verified = self.geolocation_client.verified_records(height).await?;

        Ok(VerifiedSet {
            accumulator: verified.accumulator,
            records: verified.records,
            node_identities: verified.node_identities,
            auxiliary: (),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::attested_snapshots::data::{
        AttestedSnapshot, AttestedSnapshotsCacheData, RetainedEntry,
    };
    use crate::support::caching::cache::test_helpers::round_trip_through_disk_cache;
    use cosmwasm_std::{Addr, Binary};
    use nym_contract_attestation::source::mock::mock_digest_snapshot;
    use nym_geolocation_contract_common::{
        AgentPermissions, LocationAttestation, LocationEntry, LocationPayload, Method, Source,
        Subject,
    };
    use nym_test_utils::helpers::dummy_ed25519_keypair;
    use std::collections::BTreeMap;

    fn location_entry(checked_at: u64, attestation: Option<LocationAttestation>) -> LocationEntry {
        LocationEntry {
            payload: LocationPayload {
                version: 1,
                content: Binary::new(b"payload".to_vec()),
            },
            checked_at,
            attestation,
        }
    }

    // The cache is persisted with bincode; every record shape, every `Option` set, must survive it.
    #[test]
    fn a_populated_geolocation_cache_round_trips_through_the_on_disk_format() -> anyhow::Result<()>
    {
        let height = Height::from(1000u32);
        let records = vec![
            GeolocationRecord::new_whitelisted_agent(
                Addr::unchecked("agent"),
                AgentPermissions {
                    can_measure: true,
                    can_relay_self_declared: true,
                },
            ),
            GeolocationRecord::new_location(
                Subject::new_nym_node(1),
                Source::Measured {
                    method: Method::IpInfo,
                    agent: Addr::unchecked("agent"),
                },
                location_entry(1_700_000_000, None),
            ),
            GeolocationRecord::new_location(
                Subject::new_nym_node(1),
                Source::SelfDeclared,
                location_entry(
                    1_700_000_100,
                    Some(LocationAttestation {
                        declared_at: 1_700_000_100,
                        signature: Binary::new(vec![7; 64]),
                    }),
                ),
            ),
            GeolocationRecord::new_location(
                Subject::new_nym_node(2),
                Source::Override,
                location_entry(1_700_000_200, None),
            ),
        ];
        let identities = BTreeMap::from([(1, *dummy_ed25519_keypair(2).public_key())]);
        let snapshot = AttestedSnapshot::new(
            mock_digest_snapshot(height).signed(&dummy_ed25519_keypair(1)),
            records.clone(),
            identities.clone(),
        );

        let mut cache = AttestedSnapshotsCacheData::<GeolocationSnapshotSource>::default();
        cache.insert_entry(RetainedEntry::new(snapshot, ()));

        let restored = round_trip_through_disk_cache(cache)?;
        let entry = restored.get_entry(height).unwrap();
        assert_eq!(entry.snapshot().records(), records.as_slice());
        assert_eq!(entry.snapshot().node_identities(), &identities);
        Ok(())
    }
}
