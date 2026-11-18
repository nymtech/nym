// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The nym-nodes the node-facing routes serve at a retained height, derived from the directory
//! records that height verified.

use crate::nym_nodes::node_view::described::described_node;
use crate::support::attested_snapshots::data::HeightReads;
use nym_api_requests::models::described::v3::NymNodeDescriptionV3;
use nym_bin_common::ip_check::is_global_ip;
use nym_directory_client::verify::DirectoryNode;
use nym_mixnet_contract_common::{NodeId, NymNodeDetails};
use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use time::OffsetDateTime;
use tracing::debug;

mod described;

/// The nodes listed at one retained height, and the node announcing each of their addresses.
pub(crate) struct NodeView {
    nodes: BTreeMap<NodeId, NymNodeDescriptionV3>,

    /// Keyed by the canonical form of each address.
    node_by_address: HashMap<IpAddr, NodeId>,
}

impl NodeView {
    /// The nodes `reads` lists, and the node announcing each of their addresses.
    pub(crate) fn new(reads: &HeightReads, unsafe_allow_illegal_ips: bool) -> Self {
        let nodes = listed_nodes(
            &reads.directory.node_entries(),
            &reads.mixnet.nym_nodes,
            reads.mixnet.current_key_rotation_id(),
            reads.block_time,
            unsafe_allow_illegal_ips,
        );
        let mut node_by_address = HashMap::new();
        for node in nodes.values() {
            for address in &node.description.host_information.ip_address {
                node_by_address.insert(address.to_canonical(), node.node_id);
            }
        }
        NodeView {
            nodes,
            node_by_address,
        }
    }

    /// The listed node announcing `address`, in either form of an IPv4-mapped address.
    pub(crate) fn node_with_address(&self, address: IpAddr) -> Option<NodeId> {
        self.node_by_address.get(&address.to_canonical()).copied()
    }

    /// The listed nodes in `subset`, in ascending node id order.
    pub(crate) fn nodes(&self, subset: NodeSubset) -> impl Iterator<Item = &NymNodeDescriptionV3> {
        self.nodes
            .values()
            .filter(move |node| subset.includes(node))
    }
}

/// The listed nodes a route serves, by the roles they declare.
#[derive(Clone, Copy)]
pub(crate) enum NodeSubset {
    All,
    Mixing,
    EntryCapable,
    ExitCapable,
}

impl NodeSubset {
    fn includes(self, node: &NymNodeDescriptionV3) -> bool {
        let declared = &node.description.declared_role;
        match self {
            NodeSubset::All => true,
            NodeSubset::Mixing => declared.mixnode,
            NodeSubset::EntryCapable => declared.entry,
            NodeSubset::ExitCapable => declared.can_operate_exit_gateway(),
        }
    }
}

/// The nodes listed at a snapshot height, by node id: the bonded nodes the directory describes
/// (see [`described_node`]) that announce only public addresses, unless
/// `unsafe_allow_illegal_ips`.
fn listed_nodes(
    directory: &BTreeMap<NodeId, DirectoryNode>,
    nym_nodes: &[NymNodeDetails],
    rotation_id: u32,
    block_time: OffsetDateTime,
    unsafe_allow_illegal_ips: bool,
) -> BTreeMap<NodeId, NymNodeDescriptionV3> {
    let mut listed = BTreeMap::new();
    for details in nym_nodes {
        let node_id = details.node_id();
        let Some(node) = directory.get(&node_id) else {
            debug!("nym-node {node_id} is not listed: it has no directory record");
            continue;
        };
        let described = match described_node(node, details, rotation_id, block_time) {
            Ok(described) => described,
            Err(err) => {
                debug!("nym-node {node_id} is not listed: {err:#}");
                continue;
            }
        };
        if !unsafe_allow_illegal_ips {
            let addresses = &described.description.host_information.ip_address;
            if let Some(address) = addresses.iter().find(|ip| !is_global_ip(ip)) {
                debug!("nym-node {node_id} is not listed: it announces the non-public address {address}");
                continue;
            }
        }
        listed.insert(node_id, described);
    }
    listed
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use crate::mixnet_contract_cache::data::test_helpers::nym_node;
    use nym_crypto::asymmetric::{ed25519, x25519};
    use nym_directory_client::verify::{DirectoryNode, DirectoryNodeEntry};
    use nym_directory_types::{
        DirectoryEntryPayload, LewesProtocolDetails, NodeInformation, NodeModes, NodePorts,
        SphinxKeys,
    };
    use nym_mixnet_contract_common::NymNodeDetails;
    use nym_node_requests::api::v1::lewes_protocol::models::LewesProtocol;
    use nym_node_requests::api::SignedLewesProtocol;
    use nym_test_utils::helpers::dummy_ed25519_keypair;
    use std::collections::BTreeMap;
    use time::OffsetDateTime;

    pub(crate) const ROTATION: u32 = 12;

    pub(crate) fn identity() -> ed25519::KeyPair {
        dummy_ed25519_keypair(1)
    }

    pub(crate) fn x25519_key() -> x25519::PublicKey {
        x25519::PublicKey::from([2; 32])
    }

    pub(crate) fn bond() -> NymNodeDetails {
        nym_node(7, identity().public_key().to_base58_string())
    }

    pub(crate) fn block_time() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1767225600).unwrap()
    }

    /// An entry and exit gateway on version 1.20.0 at a public address, publishing its noise key.
    pub(crate) fn node_information() -> NodeInformation {
        NodeInformation {
            binary_version: "1.20.0".to_string(),
            hostname: Some("node.example.com".to_string()),
            ip_addresses: vec!["1.2.3.4".to_string()],
            cosmos_address: "n1node".to_string(),
            location: Some("PL".to_string()),
            ports: Some(NodePorts {
                verloc_port: 1790,
                mix_port: 1789,
                ws_port: 9000,
                wss_port: None,
            }),
            modes: Some(NodeModes {
                mixnode: false,
                entry: true,
                exit: true,
                wireguard_enabled: false,
            }),
            x25519_noise_key: x25519_key().to_bytes().to_vec(),
            accepted_operator_terms_and_conditions: true,
        }
    }

    /// Lewes Protocol details carrying the node's own signature over them.
    pub(crate) fn signed_lewes_protocol() -> LewesProtocolDetails {
        let model = LewesProtocol::new(41264, 51264, x25519_key().into(), Default::default());
        let signed = SignedLewesProtocol::new(model, identity().private_key()).unwrap();

        LewesProtocolDetails {
            x25519_public_key: x25519_key().to_bytes().to_vec(),
            kem_key_digests: BTreeMap::new(),
            control_port: 41264,
            data_port: 51264,
            legacy_http_api_signature: signed.signature.to_bytes().to_vec(),
        }
    }

    /// Sphinx keys for each of `rotations`.
    pub(crate) fn sphinx_keys(rotations: &[u32]) -> SphinxKeys {
        SphinxKeys {
            keys: rotations
                .iter()
                .map(|rotation| (*rotation, x25519_key().to_bytes().to_vec()))
                .collect(),
        }
    }

    pub(crate) fn publish<T: DirectoryEntryPayload>(node: &mut DirectoryNode, payload: &T) {
        node.known_labels.insert(
            T::LABEL,
            DirectoryNodeEntry {
                data: payload.to_canonical_bytes(),
                updated_at_height: 1,
                sequence: 1,
                signature: Vec::new(),
            },
        );
    }

    /// A node whose entries all verify, with a sphinx key for [`ROTATION`].
    pub(crate) fn verified_node(
        information: &NodeInformation,
        lewes_protocol: &LewesProtocolDetails,
    ) -> DirectoryNode {
        let mut node = DirectoryNode::new(true);
        publish(&mut node, information);
        publish(&mut node, &sphinx_keys(&[ROTATION]));
        publish(&mut node, lewes_protocol);
        node
    }
}

#[cfg(test)]
mod tests {
    use super::test_helpers::{
        block_time, bond, node_information, publish, signed_lewes_protocol, sphinx_keys,
        verified_node, ROTATION,
    };
    use super::*;
    use nym_api_requests::models::described::v3::{mock_nym_node_description, DeclaredRolesV3};

    #[test]
    fn a_node_with_an_entry_that_does_not_verify_is_not_listed() {
        let mut node = verified_node(&node_information(), &signed_lewes_protocol());
        assert!(is_listed(&bond(), node.clone(), false));

        node.verified = false;

        assert!(!is_listed(&bond(), node, false));
    }

    #[test]
    fn a_node_without_a_sphinx_key_for_the_rotation_is_not_listed() {
        let mut node = verified_node(&node_information(), &signed_lewes_protocol());

        publish(&mut node, &sphinx_keys(&[ROTATION - 1]));
        assert!(!is_listed(&bond(), node.clone(), false));

        // its key for the rotation reaches the directory by a later snapshot
        publish(&mut node, &sphinx_keys(&[ROTATION - 1, ROTATION]));
        assert!(is_listed(&bond(), node, false));
    }

    #[test]
    fn an_unbonding_node_leaves_at_the_next_snapshot() {
        let earlier = verified_node(&node_information(), &signed_lewes_protocol());
        assert!(is_listed(&bond(), earlier, false));

        // the unbond transaction removed its entries, while its bond stays until the epoch ends
        let mut unbonding = bond();
        unbonding.bond_information.is_unbonding = true;
        let later = DirectoryNode::new(false);

        assert!(!is_listed(&unbonding, later, false));
    }

    #[test]
    fn a_private_address_keeps_a_node_out() {
        assert!(!is_listed(&bond(), private_address_node(), false));
    }

    #[test]
    fn the_override_lists_a_node_at_a_private_address() {
        assert!(is_listed(&bond(), private_address_node(), true));
    }

    #[test]
    fn a_subset_holds_the_nodes_declaring_its_role_in_ascending_id_order() {
        let view = NodeView {
            nodes: BTreeMap::from([
                declaring(4, true, false, false),
                declaring(1, false, true, true),
                declaring(3, false, false, true),
                declaring(2, false, true, false),
            ]),
            node_by_address: HashMap::new(),
        };
        let ids = |subset| {
            view.nodes(subset)
                .map(|node| node.node_id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids(NodeSubset::All), [1, 2, 3, 4]);
        assert_eq!(ids(NodeSubset::Mixing), [4]);
        assert_eq!(ids(NodeSubset::EntryCapable), [1, 2]);
        assert_eq!(ids(NodeSubset::ExitCapable), [1, 3]);
    }

    /// A node announcing `10.0.0.5`.
    fn private_address_node() -> DirectoryNode {
        let mut information = node_information();
        information.ip_addresses = vec!["10.0.0.5".to_string()];
        verified_node(&information, &signed_lewes_protocol())
    }

    /// Whether `details`'s node is listed when the directory holds `node` for it.
    fn is_listed(
        details: &NymNodeDetails,
        node: DirectoryNode,
        unsafe_allow_illegal_ips: bool,
    ) -> bool {
        let node_id = details.node_id();
        let directory = BTreeMap::from([(node_id, node)]);
        listed_nodes(
            &directory,
            std::slice::from_ref(details),
            ROTATION,
            block_time(),
            unsafe_allow_illegal_ips,
        )
        .contains_key(&node_id)
    }

    /// A listed node declaring the given modes, both exit roles following its exit mode.
    fn declaring(
        node_id: NodeId,
        mixnode: bool,
        entry: bool,
        exit: bool,
    ) -> (NodeId, NymNodeDescriptionV3) {
        let mut node = mock_nym_node_description(node_id.into());
        node.node_id = node_id;
        node.description.declared_role = DeclaredRolesV3 {
            mixnode,
            entry,
            exit_nr: exit,
            exit_ipr: exit,
        };
        (node_id, node)
    }
}
