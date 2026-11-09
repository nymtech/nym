// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::attested_snapshots::source::{AttestedSnapshotSource, VerifiedSet};
use crate::support::chain_anchor::{SharedChainAnchor, SharedContractAnchor};
use anyhow::Context;
use async_trait::async_trait;
use nym_contract_anchor::error::VerificationError;
use nym_contract_attestation::DirectoryEntryRecord;
use nym_directory_client::client::DirectoryClient;
use nym_directory_client::key::digest_item_key;
use nym_validator_client::nyxd::contract_traits::NymContractsProvider;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::QueryHttpRpcNyxdClient;
use tendermint::block::Height;

/// The whole directory at a height, verified through `DirectoryClient`.
pub(crate) struct DirectorySnapshotSource {
    contract_address: AccountId,

    directory_client: DirectoryClient<SharedContractAnchor, QueryHttpRpcNyxdClient>,
}

impl DirectorySnapshotSource {
    pub(crate) fn new(chain_anchor: &SharedChainAnchor) -> anyhow::Result<Self> {
        let query_client = chain_anchor.query_client().clone_query_client();
        let contract_address = query_client
            .directory_contract_address()
            .context("failed to get directory contract address")?
            .clone();
        let anchor = chain_anchor.contract_anchor(contract_address.clone(), digest_item_key());

        Ok(DirectorySnapshotSource {
            contract_address,
            directory_client: DirectoryClient::new(anchor, query_client),
        })
    }
}

#[async_trait]
impl AttestedSnapshotSource for DirectorySnapshotSource {
    type Record = DirectoryEntryRecord;
    type Auxiliary = ();

    const NAME: &'static str = "directory";

    fn contract_address(&self) -> &AccountId {
        &self.contract_address
    }

    async fn verified_set(
        &self,
        height: Height,
        _previous: Option<Self::Auxiliary>,
    ) -> Result<VerifiedSet<Self>, VerificationError> {
        let directory = self.directory_client.verified_directory(height).await?;

        Ok(VerifiedSet {
            accumulator: directory.directory.accumulator,
            records: directory.records,
            node_identities: directory.node_identities,
            auxiliary: (),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::attested_snapshots::data::AttestedSnapshotsCacheData;
    use crate::support::caching::cache::test_helpers::read_disk_cache_as;
    use nym_contract_attestation::source::mock::mock_digest_snapshot;
    use nym_contract_attestation::SignedDigestSnapshot;
    use nym_crypto::asymmetric::ed25519;
    use nym_directory_contract_common::CuratedEntry;
    use nym_mixnet_contract_common::NodeId;
    use nym_test_utils::helpers::dummy_ed25519_keypair;
    use serde::Serialize;
    use std::collections::BTreeMap;

    // the directory cache as it was persisted before the attested snapshots move
    #[derive(Serialize)]
    struct OldNymDirectoryCacheData {
        last_polled_height: Height,
        directory: BTreeMap<Height, OldCachedDirectory>,
    }

    #[derive(Serialize)]
    struct OldCachedDirectory {
        raw: OldRawCachedDirectory,
        parsed_directory: OldParsedDirectory,
    }

    #[derive(Serialize)]
    struct OldRawCachedDirectory {
        digest_snapshot: SignedDigestSnapshot,
        records: Vec<DirectoryEntryRecord>,
        node_identities: BTreeMap<NodeId, ed25519::PublicKey>,
    }

    #[derive(Serialize)]
    struct OldParsedDirectory {}

    fn old_entry(height: u32) -> OldCachedDirectory {
        let keys = dummy_ed25519_keypair(height as u64);
        OldCachedDirectory {
            raw: OldRawCachedDirectory {
                digest_snapshot: mock_digest_snapshot(Height::from(height)).signed(&keys),
                records: vec![DirectoryEntryRecord::new_curated(
                    format!("nym-api-{height}"),
                    CuratedEntry::try_from_bytes(b"curated-payload").unwrap(),
                )],
                node_identities: BTreeMap::from([(1, *keys.public_key())]),
            },
            parsed_directory: OldParsedDirectory {},
        }
    }

    #[test]
    fn a_cache_file_in_the_old_directory_layout_decodes() -> anyhow::Result<()> {
        let heights = [800u32, 900, 1000];
        let old = OldNymDirectoryCacheData {
            last_polled_height: Height::from(1050u32),
            directory: heights
                .iter()
                .map(|h| (Height::from(*h), old_entry(*h)))
                .collect(),
        };

        let decoded: AttestedSnapshotsCacheData<DirectorySnapshotSource> = read_disk_cache_as(old)?;

        // every entry decodes in full, so no field was read out of alignment
        for h in heights {
            let expected = old_entry(h).raw;
            let snapshot = decoded.get_entry(Height::from(h)).unwrap().snapshot();
            assert_eq!(
                snapshot.digest_snapshot().snapshot,
                expected.digest_snapshot.snapshot
            );
            assert_eq!(
                snapshot.digest_snapshot().signer,
                expected.digest_snapshot.signer
            );
            assert_eq!(snapshot.records(), expected.records.as_slice());
            assert_eq!(snapshot.node_identities(), &expected.node_identities);
        }

        // and so did the tip: a lag of 50 promotes 1000, one more block holds it back
        assert_eq!(
            decoded.most_recent_entry(50).unwrap().snapshot().height(),
            Height::from(1000u32)
        );
        assert_eq!(
            decoded.most_recent_entry(51).unwrap().snapshot().height(),
            Height::from(900u32)
        );
        Ok(())
    }
}
