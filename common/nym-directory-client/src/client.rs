// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The directory retrieval client: fetches the whole directory at a single height,
//! proves it against a trust anchor, and attributes each entry to its author.

use crate::anchor::TrustAnchor;
use crate::error::VerificationError;
use crate::key::{curated_entry_key, node_entry_key};
use crate::proof::{ProvenPresence, WASM_STORE_PATH, verify_wasm_store_presence};
use crate::verify::{ProvenNodeEntry, VerifiedDirectoryWithIdentities, node_signature_verifies};
use nym_contract_anchor::whole_set::verify_chain_read;
use nym_crypto::asymmetric::ed25519;
use nym_directory_contract_common::{CuratedEntry, NodeEntry};
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::nyxd::contract_traits::{
    NymContractsProvider, PinnedDirectoryQueryClient, PinnedMixnetQueryClient,
};
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{CosmWasmClient, Height};
use tracing::info;

/// A verifiable directory reader. Composes a trust anchor (which produces a digest to
/// trust at a height) with height-pinned chain queries; the anchor can be swapped
/// (proven vs a future attested / light-client anchor) without touching this reader.
pub struct DirectoryClient<A, C> {
    anchor: A,
    client: C,
}

impl<A, C> DirectoryClient<A, C> {
    pub fn client(&self) -> &C {
        &self.client
    }

    /// The anchor every read is verified against. A producer takes the `app_hash` it signs
    /// from here, so it comes from the anchor that established the digest.
    pub fn anchor(&self) -> &A {
        &self.anchor
    }
}

impl<A, C> DirectoryClient<A, C>
where
    A: TrustAnchor + Sync,
    C: CosmWasmClient
        + NymContractsProvider
        + PinnedDirectoryQueryClient
        + PinnedMixnetQueryClient
        + Sync,
{
    pub fn new(anchor: A, client: C) -> Self {
        DirectoryClient { anchor, client }
    }

    /// Retrieve and verify the complete directory at `height`.
    ///
    /// The digest proof, every entry page and the bond set are read at the SAME `height`, so
    /// a write committed mid-pagination cannot interleave into a set that matches no digest.
    /// Node identities come from the live (though unproven) bond query: this path has no
    /// trusted identity hash to check them against. Fails closed: any anchor / query error,
    /// or a recomputed digest that does not match the proven one, returns an error rather
    /// than unverified data.
    pub async fn verified_directory(
        &self,
        height: Height,
    ) -> Result<VerifiedDirectoryWithIdentities, VerificationError> {
        let records = self.client.get_all_directory_entries_at_height(height);
        let verified = verify_chain_read(&self.anchor, &self.client, height, records).await?;

        info!("verified directory at height {height}");
        Ok(verified.into())
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
    ) -> Result<Option<ProvenNodeEntry>, VerificationError> {
        let directory_contract = self
            .client
            .directory_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("directory contract"))?;

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
                    .map_err(|e| VerificationError::MalformedEntry(e.to_string()))?;
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
    ) -> Result<Option<Vec<u8>>, VerificationError> {
        let directory_contract = self
            .client
            .directory_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("directory contract"))?;

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
                    .map_err(|e| VerificationError::MalformedEntry(e.to_string()))?;
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
    ) -> Result<Option<ed25519::PublicKey>, VerificationError> {
        let res = self
            .client
            .get_nymnode_details_at_height(node_id, height)
            .await?;

        let Some(details) = res.details else {
            return Ok(None);
        };
        Ok(ed25519::PublicKey::from_base58_string(details.bond_information.identity()).ok())
    }
}
