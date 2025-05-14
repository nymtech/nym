// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::legacy::{LegacyMixNodeBondWithLayer, LegacyMixNodeDetailsWithLayer};
use nym_api_requests::models::NymNodeData;
use nym_config::defaults::DEFAULT_NYM_NODE_HTTP_PORT;
use nym_mixnet_contract_common::mixnode::LegacyPendingMixNodeChanges;
use nym_mixnet_contract_common::{
    Gateway, GatewayBond, LegacyMixLayer, MixNode, MixNodeBond, NymNodeDetails,
};
use rand::prelude::SliceRandom;
use rand::rngs::OsRng;
use std::net::{IpAddr, ToSocketAddrs};
use std::str::FromStr;

pub(crate) fn legacy_host_to_ips_and_hostname(
    legacy: &str,
) -> Option<(Vec<IpAddr>, Option<String>)> {
    if let Ok(ip) = IpAddr::from_str(legacy) {
        return Some((vec![ip], None));
    }

    let resolved = (legacy, 1789u16)
        .to_socket_addrs()
        .ok()?
        .collect::<Vec<_>>();
    Some((
        resolved.into_iter().map(|s| s.ip()).collect(),
        Some(legacy.to_string()),
    ))
}

pub(crate) fn to_legacy_mixnode(
    nym_node: &NymNodeDetails,
    description: &NymNodeData,
) -> LegacyMixNodeDetailsWithLayer {
    let layer_choices = [
        LegacyMixLayer::One,
        LegacyMixLayer::Two,
        LegacyMixLayer::Three,
    ];
    let mut rng = OsRng;

    // slap a random layer on it because legacy clients don't understand a concept of layerless mixnodes
    // SAFETY: the slice is not empty so the unwrap is fine
    #[allow(clippy::unwrap_used)]
    let layer = layer_choices.choose(&mut rng).copied().unwrap();

    LegacyMixNodeDetailsWithLayer {
        bond_information: LegacyMixNodeBondWithLayer {
            bond: MixNodeBond {
                mix_id: nym_node.node_id(),
                owner: nym_node.bond_information.owner.clone(),
                original_pledge: nym_node.bond_information.original_pledge.clone(),
                mix_node: MixNode {
                    host: nym_node.bond_information.node.host.clone(),
                    mix_port: description.mix_port(),
                    verloc_port: description.verloc_port(),
                    http_api_port: nym_node
                        .bond_information
                        .node
                        .custom_http_port
                        .unwrap_or(DEFAULT_NYM_NODE_HTTP_PORT),
                    sphinx_key: description
                        .host_information
                        .keys
                        .current_x25519_sphinx_key
                        .public_key
                        .to_base58_string(),
                    identity_key: nym_node.bond_information.node.identity_key.clone(),
                    version: description.build_information.build_version.clone(),
                },
                proxy: None,
                bonding_height: nym_node.bond_information.bonding_height,
                is_unbonding: nym_node.bond_information.is_unbonding,
            },
            layer,
        },
        rewarding_details: nym_node.rewarding_details.clone(),
        pending_changes: LegacyPendingMixNodeChanges {
            pledge_change: nym_node.pending_changes.pledge_change,
        },
    }
}

pub(crate) fn to_legacy_gateway(
    nym_node: &NymNodeDetails,
    description: &NymNodeData,
) -> GatewayBond {
    GatewayBond {
        pledge_amount: nym_node.bond_information.original_pledge.clone(),
        owner: nym_node.bond_information.owner.clone(),
        block_height: nym_node.bond_information.bonding_height,
        gateway: Gateway {
            host: nym_node.bond_information.node.host.clone(),
            mix_port: description.mix_port(),
            clients_port: description.mixnet_websockets.ws_port,
            location: description
                .auxiliary_details
                .location
                .map(|c| c.to_string())
                .unwrap_or_default(),
            sphinx_key: description
                .host_information
                .keys
                .current_x25519_sphinx_key
                .public_key
                .to_base58_string(),
            identity_key: nym_node.bond_information.node.identity_key.clone(),
            version: description.build_information.build_version.clone(),
        },
        proxy: None,
    }
}
