// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, anyhow, bail};
use nym_api_requests::models::{
    AuthenticatorDetailsV2, DeclaredRolesV2, DescribedNodeTypeV2, HostInformationV2,
    IpPacketRouterDetailsV2, LewesProtocolDetailsV1, NetworkRequesterDetailsV1,
    NetworkRequesterDetailsV2, NymNodeDataV2, OffsetDateTimeJsonSchemaWrapper, WebSocketsV2,
    WireguardDetailsV2,
};
use nym_authenticator_requests::AuthenticatorVersion;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_crypto::asymmetric::x25519;
use nym_http_api_client::UserAgent;
use nym_kkt_ciphersuite::SignatureScheme;
use nym_kkt_ciphersuite::{KEM, KEMKeyDigests};
use nym_network_defaults::DEFAULT_NYM_NODE_HTTP_PORT;
use nym_node_requests::api::client::NymNodeApiClientExt;
use nym_node_requests::api::v1::node::models::AuxiliaryDetails as NodeAuxiliaryDetails;
use nym_sdk::mixnet::NodeIdentity;
use nym_sdk::mixnet::Recipient;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::models::NymNodeDescriptionV2;
use rand::prelude::IteratorRandom;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use time::OffsetDateTime;
use tracing::{debug, info, warn};
use url::Url;

// in the old behaviour we were getting all skimmed nodes to retrieve performance
// that was ultimately unused
// should we want to use it again, the code is commented out below
//
// #[derive(Clone)]
// pub struct DescribedNodeWithPerformance {
//     pub(crate) described: NymNodeDescription,
//     // in old behaviour there was no filtering here,
//     // but in case that ever changes, this value is available
//     pub(crate) performance: u8,
// }
//
// impl DescribedNodeWithPerformance {
//     pub fn identity(&self) -> NodeIdentity {
//         self.described.ed25519_identity_key()
//     }
//
//     pub fn to_testable_node(&self) -> anyhow::Result<TestedNodeDetails> {
//         let exit_router_address = self
//             .described
//             .description
//             .ip_packet_router
//             .as_ref()
//             .map(|ipr| ipr.address.parse().context("malformed ipr address"))
//             .transpose()?;
//         let authenticator_address = self
//             .described
//             .description
//             .authenticator
//             .as_ref()
//             .map(|ipr| {
//                 ipr.address
//                     .parse()
//                     .context("malformed authenticator address")
//             })
//             .transpose()?;
//         let authenticator_version = AuthenticatorVersion::from(
//             self.described
//                 .description
//                 .build_information
//                 .build_version
//                 .as_str(),
//         );
//         let ip_address = self
//             .described
//             .description
//             .host_information
//             .ip_address
//             .first()
//             .copied();
//
//         Ok(TestedNodeDetails {
//             identity: self.identity(),
//             exit_router_address,
//             authenticator_address,
//             authenticator_version,
//             ip_address,
//         })
//     }
// }

#[derive(Clone)]
pub struct DirectoryNode {
    described: NymNodeDescriptionV2,
}

impl DirectoryNode {
    pub fn identity(&self) -> NodeIdentity {
        self.described.ed25519_identity_key()
    }

    pub fn to_testable_node(&self) -> anyhow::Result<TestedNodeDetails> {
        let description = &self.described.description;

        let exit_router_address = description
            .ip_packet_router
            .as_ref()
            .map(|ipr| ipr.address.parse().context("malformed ipr address"))
            .transpose()?;
        let authenticator_address = description
            .authenticator
            .as_ref()
            .map(|ipr| {
                ipr.address
                    .parse()
                    .context("malformed authenticator address")
            })
            .transpose()?;
        let authenticator_version =
            AuthenticatorVersion::from(description.build_information.build_version.as_str());
        let ip_address = description
            .host_information
            .ip_address
            .first()
            .copied()
            .ok_or_else(|| anyhow!("no ip address known"))?;

        let lp_data = match (
            description.lewes_protocol.clone(),
            description.host_information.keys.x25519_versioned_noise,
        ) {
            (Some(lp_data), Some(noise_key)) => Some(TestedNodeLpDetails {
                address: SocketAddr::new(ip_address, lp_data.control_port),
                expected_kem_key_hashes: lp_data.kem_keys()?,
                expected_signing_key_hashes: lp_data.signing_keys()?,
                x25519: noise_key.x25519_pubkey,
            }),
            _ => None,
        };
        let network_requester_details = self.described.description.network_requester.clone();

        Ok(TestedNodeDetails {
            identity: self.identity(),
            exit_router_address,
            network_requester_details,
            authenticator_address,
            authenticator_version,
            ip_address: Some(ip_address),
            lp_data,
        })
    }
}

/// Query a gateway directly by address using its self-described HTTP API endpoints.
/// This bypasses the need for directory service lookup.
///
/// # Arguments
/// * `address` - The address of the gateway (IP, IP:PORT, or HOST:PORT format)
///
/// # Returns
/// A `DirectoryNode` containing all gateway metadata, or an error if the query fails
pub async fn query_gateway_by_ip(address: String) -> anyhow::Result<DirectoryNode> {
    info!("Querying gateway directly at address: {address}");

    // Parse the address to check if it contains a port
    let addresses_to_try = if address.contains(':') {
        // Address already has port specified, use it directly
        vec![format!("http://{address}"), format!("https://{address}")]
    } else {
        // No port specified, try multiple ports in order of likelihood
        vec![
            format!("http://{address}:{DEFAULT_NYM_NODE_HTTP_PORT}"), // Standard port 8080
            format!("https://{address}"),                             // HTTPS proxy (443)
            format!("http://{address}"),                              // HTTP proxy (80)
        ]
    };

    let user_agent: UserAgent = nym_bin_common::bin_info_local_vergen!().into();
    let mut last_error = None;

    for address in addresses_to_try {
        debug!("Trying to connect to gateway at: {address}");

        // Build client with timeout
        let client = match nym_node_requests::api::Client::builder(address.clone()) {
            Ok(builder) => match builder
                .with_timeout(Duration::from_secs(5))
                .no_hickory_dns()
                .with_user_agent(user_agent.clone())
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to build client for {}: {}", address, e);
                    last_error = Some(e.into());
                    continue;
                }
            },
            Err(e) => {
                warn!("Failed to create client builder for {}: {}", address, e);
                last_error = Some(e.into());
                continue;
            }
        };

        // Check if the node is up
        match client.get_health().await {
            Ok(health) if health.status.is_up() => {
                info!("Successfully connected to gateway at {}", address);

                // Query all required metadata
                let host_info_result = client.get_host_information().await;
                let roles_result = client.get_roles().await;
                let build_info_result = client.get_build_information().await;
                let aux_details_result = client.get_auxiliary_details().await;
                let websockets_result = client.get_mixnet_websockets().await;
                let lp_result = client.get_lewes_protocol().await;

                // These are optional, so we use ok() to ignore errors
                let ipr_result = client.get_ip_packet_router().await.ok();
                let authenticator_result = client.get_authenticator().await.ok();
                let wireguard_result = client.get_wireguard().await.ok();

                // Check required fields
                let host_info = host_info_result.context("Failed to get host information")?;
                let roles = roles_result.context("Failed to get roles")?;
                let build_info = build_info_result.context("Failed to get build information")?;
                let aux_details: NodeAuxiliaryDetails = aux_details_result.unwrap_or_default();
                let websockets = websockets_result.context("Failed to get websocket info")?;

                // Verify node signature
                if !host_info.verify_host_information() {
                    bail!("Gateway host information signature verification failed");
                }

                // Verify it's actually a gateway
                if !roles.gateway_enabled {
                    bail!("Node at {} is not configured as an entry gateway", address);
                }

                // Convert to our internal types
                let network_requester: Option<NetworkRequesterDetailsV2> = None; // Not needed for LP testing
                let ip_packet_router: Option<IpPacketRouterDetailsV2> =
                    ipr_result.map(|ipr| IpPacketRouterDetailsV2 {
                        address: ipr.address,
                    });
                let authenticator: Option<AuthenticatorDetailsV2> =
                    authenticator_result.map(|auth| AuthenticatorDetailsV2 {
                        address: auth.address,
                    });
                #[allow(deprecated)]
                let wireguard: Option<WireguardDetailsV2> =
                    wireguard_result.map(|wg| WireguardDetailsV2 {
                        port: wg.tunnel_port, // Use tunnel_port for deprecated port field
                        tunnel_port: wg.tunnel_port,
                        metadata_port: wg.metadata_port,
                        public_key: wg.public_key,
                    });

                let lp: Option<LewesProtocolDetailsV1> = lp_result.ok().map(Into::into);

                // Construct NymNodeData
                let node_data = NymNodeDataV2 {
                    last_polled: OffsetDateTimeJsonSchemaWrapper(OffsetDateTime::now_utc()),
                    host_information: HostInformationV2 {
                        ip_address: host_info.data.ip_address,
                        hostname: host_info.data.hostname,
                        keys: host_info.data.keys.into(),
                    },
                    declared_role: DeclaredRolesV2 {
                        mixnode: roles.mixnode_enabled,
                        entry: roles.gateway_enabled,
                        exit_nr: roles.network_requester_enabled,
                        exit_ipr: roles.ip_packet_router_enabled,
                    },
                    auxiliary_details: aux_details.into(),
                    build_information: BinaryBuildInformationOwned {
                        binary_name: build_info.binary_name,
                        build_timestamp: build_info.build_timestamp,
                        build_version: build_info.build_version,
                        commit_sha: build_info.commit_sha,
                        commit_timestamp: build_info.commit_timestamp,
                        commit_branch: build_info.commit_branch,
                        rustc_version: build_info.rustc_version,
                        rustc_channel: build_info.rustc_channel,
                        cargo_triple: build_info.cargo_triple,
                        cargo_profile: build_info.cargo_profile,
                    },
                    network_requester,
                    ip_packet_router,
                    authenticator,
                    wireguard,
                    lewes_protocol: lp,
                    mixnet_websockets: WebSocketsV2 {
                        ws_port: websockets.ws_port,
                        wss_port: websockets.wss_port,
                    },
                };

                // Create NymNodeDescription
                let described = NymNodeDescriptionV2 {
                    node_id: 0, // We don't have a node_id from direct query
                    contract_node_type: DescribedNodeTypeV2::NymNode, // All new nodes are NymNode type
                    description: node_data,
                };

                return Ok(DirectoryNode { described });
            }
            Ok(_) => {
                warn!("Gateway at {} is not healthy", address);
                last_error = Some(anyhow!("Gateway is not healthy"));
            }
            Err(e) => {
                warn!("Health check failed for {}: {}", address, e);
                last_error = Some(e.into());
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("Failed to connect to gateway at {}", address)))
}

pub struct NymApiDirectory {
    // nodes: HashMap<NodeIdentity, DescribedNodeWithPerformance>,
    nodes: HashMap<NodeIdentity, DirectoryNode>,
}

impl NymApiDirectory {
    // obtain all needed directory information on genesis
    pub async fn new(api_url: Url) -> anyhow::Result<Self> {
        let user_agent: UserAgent = nym_bin_common::bin_info_local_vergen!().into();
        let api_client = nym_http_api_client::Client::builder(api_url)
            .context("malformed nym api url")?
            .with_user_agent(user_agent)
            .build()
            .context("failed to build nym api client")?;

        debug!("Fetching all described nodes from nym-api...");
        let described_nodes = api_client
            .get_all_described_nodes_v2()
            .await
            .context("nym api query failure")?;

        // let skimmed_nodes = api_client
        //     .get_all_basic_nodes_with_metadata()
        //     .await
        //     .context("nym api query failure")?;
        //
        // let performances = skimmed_nodes
        //     .nodes
        //     .into_iter()
        //     .map(|n| (n.node_id, n.performance))
        //     .collect::<HashMap<_, _>>();
        //
        // let mut nodes = HashMap::new();
        // for described_node in described_nodes {
        //     let identity = described_node.ed25519_identity_key();
        //     let Some(performance) = performances.get(&described_node.node_id) else {
        //         tracing::warn!(
        //             "Failed to append mixnet_performance, node {identity} not found among the skimmed nodes",
        //         );
        //         continue;
        //     };
        //     let info = DescribedNodeWithPerformance {
        //         described: described_node,
        //         performance: performance.round_to_integer(),
        //     };
        //     nodes.insert(identity, info);
        // }

        let nodes = described_nodes
            .into_iter()
            .map(|described| {
                (
                    described.ed25519_identity_key(),
                    DirectoryNode { described },
                )
            })
            .collect();

        Ok(NymApiDirectory { nodes })
    }

    pub fn random_exit_with_ipr(&self) -> anyhow::Result<NodeIdentity> {
        info!("Selecting random gateway with IPR enabled");
        self.nodes
            .iter()
            .filter(|(_, n)| n.described.description.ip_packet_router.is_some())
            .choose(&mut rand::thread_rng())
            .context("no gateways running IPR available")
            .map(|(id, _)| *id)
    }

    pub fn random_exit_with_nr(&self) -> anyhow::Result<NodeIdentity> {
        info!("Selecting random gateway with NR enabled");
        self.nodes
            .iter()
            .filter(|(_, n)| n.described.description.ip_packet_router.is_some())
            .choose(&mut rand::thread_rng())
            .context("no gateways running NR available")
            .map(|(id, _)| *id)
    }

    pub fn random_entry_gateway(&self) -> anyhow::Result<NodeIdentity> {
        info!("Selecting random entry gateway");
        self.nodes
            .iter()
            .filter(|(_, n)| n.described.description.declared_role.entry)
            .choose(&mut rand::thread_rng())
            .context("no entry gateways available")
            .map(|(id, _)| *id)
    }

    pub fn get_nym_node(&self, identity: NodeIdentity) -> anyhow::Result<DirectoryNode> {
        self.nodes
            .get(&identity)
            .cloned()
            .ok_or_else(|| anyhow!("did not find node {identity}"))
    }

    pub fn entry_gateway(&self, identity: &NodeIdentity) -> anyhow::Result<DirectoryNode> {
        let Some(maybe_entry) = self.nodes.get(identity).cloned() else {
            bail!("{identity} does not exist")
        };
        if !maybe_entry.described.description.declared_role.entry {
            bail!("{identity} is not an entry node")
        };
        Ok(maybe_entry)
    }

    pub fn exit_gateway_nr(&self, identity: &NodeIdentity) -> anyhow::Result<DirectoryNode> {
        let Some(maybe_entry) = self.nodes.get(identity).cloned() else {
            bail!("{identity} not found in directory")
        };
        if !maybe_entry.described.description.declared_role.exit_nr {
            bail!("{identity} doesn't support exit NR mode")
        };
        Ok(maybe_entry)
    }
}

#[derive(Default, Debug)]
pub enum TestedNode {
    #[default]
    SameAsEntry,
    Custom {
        identity: NodeIdentity,
        shares_entry: bool,
    },
}

impl TestedNode {
    pub fn is_same_as_entry(&self) -> bool {
        matches!(
            self,
            TestedNode::SameAsEntry
                | TestedNode::Custom {
                    shares_entry: true,
                    ..
                }
        )
    }
}

#[derive(Debug, Clone)]
pub struct TestedNodeDetails {
    pub identity: NodeIdentity,
    pub exit_router_address: Option<Recipient>,
    pub network_requester_details: Option<NetworkRequesterDetailsV1>,
    pub authenticator_address: Option<Recipient>,
    pub authenticator_version: AuthenticatorVersion,
    pub ip_address: Option<IpAddr>,
    pub lp_data: Option<TestedNodeLpDetails>,
}

#[derive(Debug, Clone)]
pub struct TestedNodeLpDetails {
    pub address: SocketAddr,
    pub expected_kem_key_hashes: HashMap<KEM, KEMKeyDigests>,
    pub expected_signing_key_hashes: HashMap<SignatureScheme, KEMKeyDigests>,
    pub x25519: x25519::PublicKey,
}

impl TestedNodeDetails {
    /// Create from CLI args (localnet mode - no HTTP query needed)
    pub fn from_cli(identity: NodeIdentity, lp_data: TestedNodeLpDetails) -> Self {
        Self {
            identity,
            ip_address: Some(lp_data.address.ip()),
            lp_data: Some(lp_data),
            network_requester_details: None,
            // These are None in localnet mode - only needed for mixnet/authenticator
            exit_router_address: None,
            authenticator_address: None,
            authenticator_version: AuthenticatorVersion::UNKNOWN,
        }
    }

    /// Check if this node has sufficient info for LP testing
    pub fn can_test_lp(&self) -> bool {
        self.lp_data.is_some()
    }

    /// Check if this node has sufficient info for mixnet testing
    pub fn can_test_mixnet(&self) -> bool {
        self.exit_router_address.is_some() || self.authenticator_address.is_some()
    }
}
