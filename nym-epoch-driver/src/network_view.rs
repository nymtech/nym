// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The selection candidates: the bonded nodes with their stake and their roles from the verified
//! directory, read together at one cadence boundary, which selection reads instead of the chain
//! at epoch time.

use anyhow::Context;
use nym_contract_anchor::anchor::TrustAnchor;
use nym_contract_anchor::anchor::contract::ContractAnchor;
use nym_contract_anchor::anchor::proven::ProvenChainAnchor;
use nym_contract_anchor::whole_set::{node_identities, verify_chain_read};
use nym_directory_client::verify::VerifiedDirectory;
use nym_directory_types::{NodeInformation, NodeModes};
use nym_mixnet_contract_common::{NodeId, NodeRewarding};
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

/// A bonded node, not unbonding, with at least one role, as selection weighs it.
pub(crate) struct SelectionCandidate {
    pub(crate) node_id: NodeId,

    /// Where the node's stake, and so its saturation, comes from.
    pub(crate) rewarding: NodeRewarding,

    pub(crate) roles: Vec<AvailableRole>,
}

pub(crate) struct NetworkView {
    /// The cadence boundary everything below was read at.
    pub(crate) height: Height,

    /// Every bonded node, not unbonding, whose verified directory record gives it a role.
    pub(crate) candidates: Vec<SelectionCandidate>,
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
        let mut roles = node_roles(&directory);

        let bonded = nodes.len();
        let candidates = nodes
            .into_iter()
            .filter_map(|node| {
                let node_id = node.node_id();
                // the epoch-end reconciliation, which runs before selection, removes it
                if node.is_unbonding() {
                    debug!("nym-node {node_id} is unbonding");
                    return None;
                }
                Some(SelectionCandidate {
                    node_id,
                    rewarding: node.rewarding_details,
                    roles: roles.remove(&node_id)?,
                })
            })
            .collect::<Vec<_>>();

        info!(
            "produced the network view at height {height}: {bonded} bonded nodes, {} with a role",
            candidates.len()
        );
        Ok(NetworkView { height, candidates })
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
