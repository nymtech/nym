// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The bonded nodes and the verified directory, read together at one cadence boundary, which
//! selection reads instead of the chain at epoch time.

use anyhow::Context;
use nym_contract_anchor::anchor::TrustAnchor;
use nym_contract_anchor::anchor::contract::ContractAnchor;
use nym_contract_anchor::anchor::proven::ProvenChainAnchor;
use nym_contract_anchor::whole_set::{node_identities, verify_chain_read};
use nym_directory_client::verify::VerifiedDirectory;
use nym_mixnet_contract_common::NymNodeDetails;
use nym_validator_client::QueryHttpRpcNyxdClient;
use nym_validator_client::nyxd::Height;
use nym_validator_client::nyxd::contract_traits::{
    PinnedDirectoryQueryClient, PinnedMixnetQueryClient,
};
use tracing::info;

/// The directory's digest, proven on the block headers of the driver's own RPC node, which it
/// always trusts.
pub(crate) type DirectoryAnchor =
    ContractAnchor<ProvenChainAnchor<QueryHttpRpcNyxdClient>, QueryHttpRpcNyxdClient>;

pub(crate) struct NetworkView {
    /// The cadence boundary everything below was read at.
    pub(crate) height: Height,

    /// Every bonded node's details: its bond, identity and stake.
    pub(crate) nodes: Vec<NymNodeDetails>,

    /// The directory, verified against its digest with the identities of `nodes`.
    pub(crate) directory: VerifiedDirectory,
}

impl NetworkView {
    /// Reads the node details and the verified directory at `height`. The details' identity
    /// keys are the map the directory is verified with, so the bond set is read once.
    pub(crate) async fn produce<A, C>(
        anchor: &A,
        client: &C,
        height: Height,
    ) -> anyhow::Result<Self>
    where
        A: TrustAnchor + Sync,
        C: PinnedMixnetQueryClient + PinnedDirectoryQueryClient + Sync,
    {
        let nodes = client
            .get_all_nymnodes_detailed_at_height(height)
            .await
            .context("failed to read the node details")?;
        let node_identities = node_identities(nodes.iter().map(|node| &node.bond_information));

        let records = client.get_all_directory_entries_at_height(height);
        let directory = VerifiedDirectory::from(
            verify_chain_read(anchor, height, records, node_identities)
                .await
                .context("failed to verify the directory")?,
        );

        info!(
            "produced the network view at height {height}: {} bonded nodes",
            nodes.len()
        );
        Ok(NetworkView {
            height,
            nodes,
            directory,
        })
    }
}
