// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Context;
use celes::Country;
use nym_api_requests::models::described::v3::{
    AnnouncePortsV3, AuthenticatorDetailsV3, DeclaredRolesV3, DescribedNodeTypeV3,
    HostInformationV3, HostKeysV3, IpPacketRouterDetailsV3, LewesProtocolDetailsV3,
    NetworkRequesterDetailsV3, NymNodeAuxiliaryDetailsV3, NymNodeDataV3, NymNodeDescriptionV3,
    SphinxKeyV3, VersionedNoiseKeyV3, WebSocketsV3, WireguardDetailsV3,
};
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_directory_client::verify::DirectoryNode;
use nym_directory_types::{
    DirectoryTypesError, LewesProtocolDetails, MixnetServiceProviders, NodeInformation, SphinxKeys,
    Wireguard,
};
use nym_mixnet_contract_common::NymNodeDetails;
use nym_node_requests::api::SignedLewesProtocol;
use nym_noise_keys::NoiseVersion;
use nym_sphinx_addressing::clients::{Recipient, RecipientBytes};
use std::net::IpAddr;
use time::OffsetDateTime;

/// Every `build_information` field the directory does not hold.
const UNKNOWN: &str = "unknown";

/// The Unix epoch, in the format nym-node reports its build timestamp in.
const UNIX_EPOCH_BUILD_TIMESTAMP: &str = "1970-01-01T00:00:00+00:00";

/// One node as the described routes serve it at a snapshot height, from its verified directory
/// entries, its bond, the key rotation in force and the height's block time.
///
/// Fails when its node information or its sphinx key for `rotation_id` is missing or malformed.
/// Every other entry is optional: one that is missing or malformed is served as absent.
pub(crate) fn described_node(
    node: &DirectoryNode,
    details: &NymNodeDetails,
    rotation_id: u32,
    block_time: OffsetDateTime,
) -> anyhow::Result<NymNodeDescriptionV3> {
    let identity: ed25519::PublicKey = details
        .bond_information
        .identity()
        .parse()
        .context("the bonded identity key is malformed")?;
    let information = node.payload::<NodeInformation>()?;
    let ip_address = information
        .ip_addresses
        .iter()
        .map(|ip| {
            ip.parse::<IpAddr>()
                .with_context(|| format!("the announced ip address '{ip}' is malformed"))
        })
        .collect::<anyhow::Result<_>>()?;
    let keys = host_keys(node, identity, rotation_id, &information)?;
    let ports = information
        .ports
        .context("the node information announces no ports")?;
    // a node announcing no modes declares no roles
    let modes = information.modes.unwrap_or_default();
    let providers = node.payload::<MixnetServiceProviders>().unwrap_or_default();

    Ok(NymNodeDescriptionV3 {
        node_id: details.node_id(),
        contract_node_type: DescribedNodeTypeV3::NymNode,
        description: NymNodeDataV3 {
            last_polled: block_time.into(),
            host_information: HostInformationV3 {
                ip_address,
                hostname: information.hostname,
                keys,
            },
            declared_role: DeclaredRolesV3 {
                mixnode: modes.mixnode,
                entry: modes.entry,
                exit_nr: modes.exit,
                exit_ipr: modes.exit,
            },
            auxiliary_details: NymNodeAuxiliaryDetailsV3 {
                location: information
                    .location
                    .and_then(|code| Country::from_alpha2(code).ok()),
                address: Some(information.cosmos_address).filter(|address| !address.is_empty()),
                announce_ports: AnnouncePortsV3 {
                    verloc_port: Some(ports.get_verloc_port()),
                    mix_port: Some(ports.get_mix_port()),
                },
                accepted_operator_terms_and_conditions: information
                    .accepted_operator_terms_and_conditions,
            },
            build_information: build_information(information.binary_version),
            network_requester: providers
                .network_requester
                .and_then(|provider| mixnet_address(provider.recipient_bytes()))
                .map(|address| NetworkRequesterDetailsV3 {
                    address,
                    uses_exit_policy: true,
                }),
            ip_packet_router: providers
                .internet_packet_router
                .and_then(|provider| mixnet_address(provider.recipient_bytes()))
                .map(|address| IpPacketRouterDetailsV3 { address }),
            authenticator: providers
                .authenticator
                .and_then(|provider| mixnet_address(provider.recipient_bytes()))
                .map(|address| AuthenticatorDetailsV3 { address }),
            wireguard: wireguard(node, modes.wireguard_enabled),
            mixnet_websockets: WebSocketsV3 {
                ws_port: ports.get_ws_port(),
                wss_port: ports.get_wss_port(),
            },
            lewes_protocol: lewes_protocol(node, &identity),
        },
    })
}

/// The identity, the sphinx key of `rotation_id` and of the next rotation when pre-announced, and
/// the noise key when published.
#[allow(deprecated)]
fn host_keys(
    node: &DirectoryNode,
    identity: ed25519::PublicKey,
    rotation_id: u32,
    information: &NodeInformation,
) -> anyhow::Result<HostKeysV3> {
    let sphinx_keys = node.payload::<SphinxKeys>()?;
    let current = sphinx_keys
        .key_for_rotation(rotation_id)
        .with_context(|| format!("no sphinx key is published for rotation {rotation_id}"))??;
    let next_rotation = rotation_id + 1;
    let pre_announced = sphinx_keys
        .key_for_rotation(next_rotation)
        .and_then(Result::ok)
        .map(|key| SphinxKeyV3 {
            rotation_id: next_rotation,
            public_key: key.into(),
        });
    // nodes publish the key alone: V1 is the only noise version
    let noise = information
        .x25519_noise_key_bytes()
        .ok()
        .map(|key| VersionedNoiseKeyV3 {
            supported_version: NoiseVersion::V1,
            x25519_pubkey: key.into(),
        });

    Ok(HostKeysV3 {
        ed25519: identity,
        x25519: current.into(),
        current_x25519_sphinx_key: SphinxKeyV3 {
            rotation_id,
            public_key: current.into(),
        },
        pre_announced_x25519_sphinx_key: pre_announced,
        x25519_versioned_noise: noise,
    })
}

/// The build information, of which the directory holds only the version.
fn build_information(build_version: String) -> BinaryBuildInformationOwned {
    BinaryBuildInformationOwned {
        binary_name: "nym-node".to_string(),
        build_timestamp: UNIX_EPOCH_BUILD_TIMESTAMP.to_string(),
        build_version,
        commit_sha: UNKNOWN.to_string(),
        commit_timestamp: UNKNOWN.to_string(),
        commit_branch: UNKNOWN.to_string(),
        rustc_version: UNKNOWN.to_string(),
        rustc_channel: UNKNOWN.to_string(),
        cargo_profile: UNKNOWN.to_string(),
        cargo_triple: UNKNOWN.to_string(),
    }
}

/// A service provider's mixnet address, when its published bytes are one.
fn mixnet_address(bytes: Result<RecipientBytes, DirectoryTypesError>) -> Option<String> {
    let recipient = Recipient::try_from_bytes(bytes.ok()?).ok()?;
    Some(recipient.to_string())
}

/// The wireguard details, when the node runs wireguard.
fn wireguard(node: &DirectoryNode, enabled: bool) -> Option<WireguardDetailsV3> {
    if !enabled {
        return None;
    }
    let wireguard = node.payload::<Wireguard>().ok()?;
    let public_key = x25519::PublicKey::from(wireguard.public_key_bytes().ok()?);

    Some(WireguardDetailsV3 {
        // the deprecated predecessor of the tunnel port
        port: wireguard.get_tunnel_port(),
        tunnel_port: wireguard.get_tunnel_port(),
        metadata_port: wireguard.get_metadata_port(),
        public_key: public_key.to_base58_string(),
    })
}

/// The Lewes Protocol details with the node's own signature over them: without one that verifies
/// there are none to serve.
fn lewes_protocol(
    node: &DirectoryNode,
    identity: &ed25519::PublicKey,
) -> Option<LewesProtocolDetailsV3> {
    let published = node.payload::<LewesProtocolDetails>().ok()?;
    let signed = LewesProtocolDetailsV3::from(SignedLewesProtocol::try_from(published).ok()?);
    signed.verify(identity).then_some(signed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nym_nodes::node_view::test_helpers::{
        block_time, bond, identity, node_information, signed_lewes_protocol, verified_node,
        ROTATION,
    };
    use nym_directory_types::NodeModes;

    #[test]
    fn the_build_version_is_the_published_one() {
        let node = verified_node(&node_information(), &signed_lewes_protocol());

        let build = described(&node).description.build_information;

        assert_eq!(build.build_version, "1.20.0");
        assert_eq!(build.binary_name, "nym-node");
        assert_eq!(build.build_timestamp, "1970-01-01T00:00:00+00:00");
        for field in [
            build.commit_sha,
            build.commit_timestamp,
            build.commit_branch,
            build.rustc_version,
            build.rustc_channel,
            build.cargo_profile,
            build.cargo_triple,
        ] {
            assert_eq!(field, "unknown");
        }
    }

    #[test]
    fn exit_without_entry_is_exit_capable() {
        let mut information = node_information();
        information.modes = Some(NodeModes {
            mixnode: false,
            entry: false,
            exit: true,
            wireguard_enabled: false,
        });
        let node = verified_node(&information, &signed_lewes_protocol());

        let roles = described(&node).description.declared_role;

        assert!(roles.exit_nr);
        assert!(roles.exit_ipr);
        assert!(!roles.entry);
    }

    #[test]
    fn a_node_not_publishing_the_noise_key_is_served_without_one() {
        let mut information = node_information();
        information.x25519_noise_key = Vec::new();
        let node = verified_node(&information, &signed_lewes_protocol());

        let keys = described(&node).description.host_information.keys;

        assert!(keys.x25519_versioned_noise.is_none());
    }

    #[test]
    fn unsigned_lewes_protocol_details_are_not_served() {
        let signed = verified_node(&node_information(), &signed_lewes_protocol());
        let served = described(&signed).description.lewes_protocol.unwrap();
        assert!(served.verify(identity().public_key()));

        let mut lewes_protocol = signed_lewes_protocol();
        lewes_protocol.legacy_http_api_signature = Vec::new();
        let unsigned = verified_node(&node_information(), &lewes_protocol);

        assert!(described(&unsigned).description.lewes_protocol.is_none());
    }

    fn described(node: &DirectoryNode) -> NymNodeDescriptionV3 {
        described_node(node, &bond(), ROTATION, block_time()).unwrap()
    }
}
