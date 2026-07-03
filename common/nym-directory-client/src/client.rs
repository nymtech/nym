// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The directory retrieval client: fetches the whole directory at a single height,
//! proves it against a trust anchor, and attributes each entry to its author.

use crate::anchor::DirectoryTrustAnchor;
use crate::error::DirectoryClientError;
use crate::verify::{
    DirectoryNode, DirectoryNodeEntry, VerifiedDirectory, node_signature_verifies,
    recompute_accumulator,
};
use nym_crypto::asymmetric::ed25519;
use nym_directory_contract_common::{
    AllEntriesPagedResponse, DirectoryEntryRecord, EntryKey, KnownLabel,
    QueryMsg as DirectoryQueryMsg,
};
use nym_mixnet_contract_common::nym_node::PagedNymNodeBondsResponse;
use nym_mixnet_contract_common::{NodeId, QueryMsg as MixnetQueryMsg};
use nym_validator_client::nyxd::{AccountId, CosmWasmClient, Height};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use tracing::error;

/// A verifiable directory reader. Composes a trust anchor (which produces a digest to
/// trust at a height) with height-pinned chain queries; the anchor can be swapped
/// (proven vs a future attested / light-client anchor) without touching this reader.
pub struct DirectoryClient<A, C> {
    anchor: A,
    client: C,
    directory_contract: AccountId,
    mixnet_contract: AccountId,
}

impl<A, C> DirectoryClient<A, C>
where
    A: DirectoryTrustAnchor + Sync,
    C: CosmWasmClient + Sync,
{
    pub fn new(
        anchor: A,
        client: C,
        directory_contract: AccountId,
        mixnet_contract: AccountId,
    ) -> Self {
        DirectoryClient {
            anchor,
            client,
            directory_contract,
            mixnet_contract,
        }
    }

    /// Retrieve and verify the complete directory at `height`.
    ///
    /// The digest proof and every entry page are read at the SAME `height`, so a write
    /// committed mid-pagination cannot interleave into a set that matches no digest.
    /// Fails closed: any anchor / query error, or a recomputed digest that does not
    /// match the proven one, returns an error rather than unverified data.
    pub async fn verified_directory(
        &self,
        height: Height,
    ) -> Result<VerifiedDirectory, DirectoryClientError> {
        // 1. establish the digest we trust at this height (proof handled by the anchor)
        let trusted = self.anchor.trusted_digest(height).await?;

        // 2. fetch the whole entry set at the SAME height
        let records = self.all_entries_at(height).await?;

        // 3. recompute locally and compare (whole-set integrity in one shot)
        if recompute_accumulator(&records) != trusted.accumulator {
            return Err(DirectoryClientError::DigestMismatch);
        }

        // 4. retrieve identity keys of all nym-nodes at this height
        let node_identities = self.all_node_identities_at(height).await?;

        // 5. attribute each entry to its author (node signature vs admin authority)
        let mut curated_entries = BTreeMap::new();
        let mut node_entries = BTreeMap::new();

        for record in records {
            match record {
                DirectoryEntryRecord::Curated { key, entry } => {
                    curated_entries.insert(key, entry.data.into());
                }
                DirectoryEntryRecord::Node {
                    node_id,
                    label,
                    entry: node_entry,
                } => {
                    // verify the node signature on the submitted data
                    let verified = match node_identities.get(&node_id) {
                        Some(identity) => {
                            node_signature_verifies(node_id, &label, &node_entry, identity)
                        }
                        None => false,
                    };
                    let entry = node_entries
                        .entry(node_id)
                        .or_insert(DirectoryNode::new(verified));
                    entry.verified &= verified;

                    let data = DirectoryNodeEntry {
                        data: node_entry.data.into(),
                        updated_at_height: node_entry.updated_at_height,
                        sequence: node_entry.sequence,
                        signature: node_entry.signature.into(),
                    };

                    if let Ok(known) = KnownLabel::from_str(&label) {
                        entry.known_labels.insert(known, data);
                    } else {
                        entry.unknown_labels.insert(label, data);
                    }
                }
            }
        }

        Ok(VerifiedDirectory {
            height: trusted.height,
            accumulator: trusted.accumulator,
            curated_entries,
            node_entries,
        })
    }

    /// Page through `AllEntries`, every request pinned to `height`.
    async fn all_entries_at(
        &self,
        height: Height,
    ) -> Result<Vec<DirectoryEntryRecord>, DirectoryClientError> {
        let mut records = Vec::new();
        let mut start_after: Option<EntryKey> = None;
        loop {
            let page: AllEntriesPagedResponse = self
                .client
                .query_contract_smart_at_height(
                    &self.directory_contract,
                    &DirectoryQueryMsg::AllEntries {
                        start_after,
                        limit: None,
                    },
                    Some(height),
                )
                .await?;

            records.extend(page.entries);
            match page.start_next_after {
                Some(cursor) => start_after = Some(cursor),
                None => break,
            }
        }
        Ok(records)
    }

    async fn all_node_identities_at(
        &self,
        height: Height,
    ) -> Result<HashMap<NodeId, ed25519::PublicKey>, DirectoryClientError> {
        let mut bonds = Vec::new();
        let mut start_after = None;
        loop {
            let page: PagedNymNodeBondsResponse = self
                .client
                .query_contract_smart_at_height(
                    &self.mixnet_contract,
                    &MixnetQueryMsg::GetNymNodeBondsPaged {
                        start_after,
                        limit: None,
                    },
                    Some(height),
                )
                .await?;

            bonds.extend(page.nodes);
            match page.start_next_after {
                Some(cursor) => start_after = Some(cursor),
                None => break,
            }
        }
        let mut identities = HashMap::new();
        for bond in bonds {
            let Ok(identity) = bond.identity().parse() else {
                // this should be impossible as otherwise we wouldn't have been able to verify
                // signatures within the mixnet contract
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
