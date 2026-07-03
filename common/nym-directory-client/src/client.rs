// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The directory retrieval client: fetches the whole directory at a single height,
//! proves it against a trust anchor, and attributes each entry to its author.

use crate::anchor::DirectoryTrustAnchor;
use crate::error::DirectoryClientError;
use crate::key::{curated_entry_key, node_entry_key};
use crate::proof::{ProvenPresence, WASM_STORE_PATH, verify_wasm_store_presence};
use crate::verify::{
    DirectoryNode, DirectoryNodeEntry, ProvenNodeEntry, VerifiedDirectory, node_signature_verifies,
    recompute_accumulator,
};
use nym_crypto::asymmetric::ed25519;
use nym_directory_contract_common::{
    AllEntriesPagedResponse, CuratedEntry, DirectoryEntryRecord, EntryKey, KnownLabel, NodeEntry,
    QueryMsg as DirectoryQueryMsg,
};
use nym_mixnet_contract_common::nym_node::{NodeDetailsResponse, PagedNymNodeBondsResponse};
use nym_mixnet_contract_common::{NodeId, QueryMsg as MixnetQueryMsg};
use nym_validator_client::nyxd::contract_traits::NymContractsProvider;
use nym_validator_client::nyxd::{CosmWasmClient, Height};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use tracing::error;

/// A verifiable directory reader. Composes a trust anchor (which produces a digest to
/// trust at a height) with height-pinned chain queries; the anchor can be swapped
/// (proven vs a future attested / light-client anchor) without touching this reader.
pub struct DirectoryClient<A, C> {
    anchor: A,
    client: C,
}

impl<A, C> DirectoryClient<A, C>
where
    A: DirectoryTrustAnchor + Sync,
    C: CosmWasmClient + NymContractsProvider + Sync,
{
    pub fn new(anchor: A, client: C) -> Self {
        DirectoryClient { anchor, client }
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

                    let data = DirectoryNodeEntry::from(node_entry);

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

    /// Retrieve and verify a single node entry `(node_id, label)` at `height` via its own
    /// ICS23 proof against the block `app_hash`.
    ///
    /// Returns `Ok(None)` when the entry is proven ABSENT (a verified non-existence proof),
    /// distinct from a verification failure (`Err`). A present entry is decoded with the
    /// contract value codec and its signature checked against the node's bonded identity
    /// key. Presence is decided by the proof shape, not by the value's emptiness.
    pub async fn verified_node_entry(
        &self,
        node_id: NodeId,
        label: &str,
        height: Height,
    ) -> Result<Option<ProvenNodeEntry>, DirectoryClientError> {
        let directory_contract = self
            .client
            .directory_contract_address()
            .ok_or(DirectoryClientError::UnavailableDirectoryContract)?;

        // reconstruct the raw key ourselves so a malicious RPC cannot substitute another key
        let key = node_entry_key(directory_contract, node_id, label);

        let res = self
            .client
            .make_raw_abci_query_with_proof(
                Some(WASM_STORE_PATH.to_owned()),
                key.clone(),
                Some(height),
            )
            .await?;

        // the app_hash comes from the trust anchor (NOT re-fetched from the same RPC that
        // served the proof), so a malicious RPC cannot substitute a self-consistent forgery
        let app_hash = self.anchor.trusted_app_hash(height).await?;

        match verify_wasm_store_presence(&res.proof.ops, app_hash.as_bytes(), &key, &res.response)?
        {
            ProvenPresence::Absent => Ok(None),
            ProvenPresence::Present => {
                let node_entry = NodeEntry::try_from_bytes(&res.response)
                    .map_err(|e| DirectoryClientError::MalformedEntry(e.to_string()))?;
                let verified = match self.node_identity_at(node_id, height).await? {
                    Some(identity) => {
                        node_signature_verifies(node_id, label, &node_entry, &identity)
                    }
                    None => false,
                };
                Ok(Some(ProvenNodeEntry {
                    entry: node_entry.into(),
                    verified,
                }))
            }
        }
    }

    /// Retrieve and verify a single curated entry by its `key` at `height` via its own
    /// ICS23 proof against the block `app_hash`.
    ///
    /// Returns `Ok(None)` when the entry is proven ABSENT (a verified non-existence proof),
    /// distinct from a verification failure (`Err`). Curated entries carry no per-entry
    /// signature - their authority is the contract admin - so a verified membership proof
    /// against the trusted app_hash is itself the authentication; the decoded payload bytes
    /// are returned. Presence is decided by the proof shape, not by the value's emptiness.
    pub async fn verified_curated_entry(
        &self,
        key: &str,
        height: Height,
    ) -> Result<Option<Vec<u8>>, DirectoryClientError> {
        let directory_contract = self
            .client
            .directory_contract_address()
            .ok_or(DirectoryClientError::UnavailableDirectoryContract)?;

        // reconstruct the raw key ourselves so a malicious RPC cannot substitute another key
        let raw_key = curated_entry_key(directory_contract, key);

        let res = self
            .client
            .make_raw_abci_query_with_proof(
                Some(WASM_STORE_PATH.to_owned()),
                raw_key.clone(),
                Some(height),
            )
            .await?;

        // the app_hash comes from the trust anchor (NOT re-fetched from the same RPC that
        // served the proof), so a malicious RPC cannot substitute a self-consistent forgery
        let app_hash = self.anchor.trusted_app_hash(height).await?;

        match verify_wasm_store_presence(
            &res.proof.ops,
            app_hash.as_bytes(),
            &raw_key,
            &res.response,
        )? {
            ProvenPresence::Absent => Ok(None),
            ProvenPresence::Present => {
                let entry = CuratedEntry::try_from_bytes(&res.response)
                    .map_err(|e| DirectoryClientError::MalformedEntry(e.to_string()))?;
                Ok(Some(entry.data.into()))
            }
        }
    }

    /// The node's ed25519 identity key from its mixnet bond at `height`, or `None` if the
    /// node cannot be resolved (absent, or a malformed on-chain key). Unbonding status is
    /// irrelevant to attribution: the identity key is unchanged and the entry was signed
    /// with it.
    async fn node_identity_at(
        &self,
        node_id: NodeId,
        height: Height,
    ) -> Result<Option<ed25519::PublicKey>, DirectoryClientError> {
        let mixnet_contract = self
            .client
            .mixnet_contract_address()
            .ok_or(DirectoryClientError::UnavailableMixnetContract)?;

        let res: NodeDetailsResponse = self
            .client
            .query_contract_smart_at_height(
                mixnet_contract,
                &MixnetQueryMsg::GetNymNodeDetails { node_id },
                Some(height),
            )
            .await?;

        let Some(details) = res.details else {
            return Ok(None);
        };
        Ok(ed25519::PublicKey::from_base58_string(details.bond_information.identity()).ok())
    }

    /// Page through `AllEntries`, every request pinned to `height`.
    async fn all_entries_at(
        &self,
        height: Height,
    ) -> Result<Vec<DirectoryEntryRecord>, DirectoryClientError> {
        let directory_contract = self
            .client
            .directory_contract_address()
            .ok_or(DirectoryClientError::UnavailableDirectoryContract)?;

        let mut records = Vec::new();
        let mut start_after: Option<EntryKey> = None;
        loop {
            let page: AllEntriesPagedResponse = self
                .client
                .query_contract_smart_at_height(
                    directory_contract,
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
        let mixnet_contract = self
            .client
            .mixnet_contract_address()
            .ok_or(DirectoryClientError::UnavailableMixnetContract)?;

        let mut bonds = Vec::new();
        let mut start_after = None;
        loop {
            let page: PagedNymNodeBondsResponse = self
                .client
                .query_contract_smart_at_height(
                    mixnet_contract,
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
