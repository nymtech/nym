// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The bonded nodes and their roles from the verified directory, read together at one cadence
//! boundary, which selection reads instead of the chain at epoch time.

use anyhow::Context;
use nym_contract_anchor::anchor::TrustAnchor;
use nym_contract_anchor::anchor::contract::ContractAnchor;
use nym_contract_anchor::anchor::proven::ProvenChainAnchor;
use nym_contract_anchor::whole_set::{node_identities, verify_chain_read};
use nym_directory_client::verify::VerifiedDirectory;
use nym_directory_types::{NodeInformation, NodeModes};
use nym_mixnet_contract_common::{NodeId, NymNodeDetails};
use nym_validator_client::QueryHttpRpcNyxdClient;
use nym_validator_client::nyxd::Height;
use nym_validator_client::nyxd::contract_traits::{
    PinnedDirectoryQueryClient, PinnedMixnetQueryClient,
};
use std::collections::HashMap;
use tracing::{debug, info};

/// The directory's digest, proven on the block headers of the driver's own RPC node, which it
/// always trusts.
pub(crate) type DirectoryAnchor =
    ContractAnchor<ProvenChainAnchor<QueryHttpRpcNyxdClient>, QueryHttpRpcNyxdClient>;

/// A role of the rewarded set a node can be selected for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AvailableRole {
    // nodes publishing the mixnode mode
    Mix,

    // nodes publishing the entry mode
    EntryGateway,

    // nodes publishing the exit mode, whatever service providers they run
    ExitGateway,
}

pub(crate) struct NetworkView {
    /// The cadence boundary everything below was read at.
    pub(crate) height: Height,

    /// Every bonded node's details: its bond, identity and stake.
    pub(crate) nodes: Vec<NymNodeDetails>,

    /// The roles of every node whose verified directory record declares any.
    pub(crate) roles: HashMap<NodeId, Vec<AvailableRole>>,
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
        let roles = node_roles(&directory);

        info!(
            "produced the network view at height {height}: {} bonded nodes, {} with a role",
            nodes.len(),
            roles.len()
        );
        Ok(NetworkView {
            height,
            nodes,
            roles,
        })
    }
}

/// Each node's roles from its `NodeInformation` record. A node whose entries do not all verify
/// against its bonded identity, or whose record is missing or does not decode, gets none.
fn node_roles(directory: &VerifiedDirectory) -> HashMap<NodeId, Vec<AvailableRole>> {
    let mut roles = HashMap::new();
    for (node_id, node) in &directory.node_entries {
        if !node.verified {
            debug!("nym-node {node_id} has no verified directory entries");
            continue;
        }
        let information = match node.payload::<NodeInformation>() {
            Some(Ok(information)) => information,
            Some(Err(err)) => {
                debug!("nym-node {node_id} publishes undecodable node information: {err}");
                continue;
            }
            None => {
                debug!("nym-node {node_id} publishes no node information");
                continue;
            }
        };
        let node_roles = information
            .modes
            .as_ref()
            .map(selectable_roles)
            .unwrap_or_default();
        if !node_roles.is_empty() {
            roles.insert(*node_id, node_roles);
        }
    }
    roles
}

/// The roles `modes` make a node selectable for, the exit role by the exit mode alone.
fn selectable_roles(modes: &NodeModes) -> Vec<AvailableRole> {
    let mut roles = Vec::new();
    if modes.mixnode {
        roles.push(AvailableRole::Mix)
    }
    if modes.entry {
        roles.push(AvailableRole::EntryGateway)
    }
    if modes.exit {
        roles.push(AvailableRole::ExitGateway)
    }
    roles
}
