// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::config::Config;
use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{BondedNymNode, NewNymNode, NodeType};
use anyhow::Context;
use futures::{StreamExt, stream};
use nym_bin_common::bin_info;
use nym_crypto::asymmetric::x25519;
use nym_network_defaults::DEFAULT_MIX_LISTENING_PORT;
use nym_node_requests::api::client::NymNodeApiClientExt;
use nym_node_requests::api::helpers::NymNodeApiClientRetriever;
use nym_node_requests::api::v1::node::models::NodeRoles;
use nym_task::ShutdownToken;
use nym_validator_client::QueryHttpRpcNyxdClient;
use nym_validator_client::models::KeyRotationId;
use nym_validator_client::nyxd::contract_traits::PagedMixnetQueryClient;
use nym_validator_client::nyxd::nym_mixnet_contract_common::NymNodeBond;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::time::{Instant, interval};
use tracing::{debug, error, info};

pub(crate) struct NodeRefresher {
    pub(crate) client: QueryHttpRpcNyxdClient,

    pub(crate) storage: NetworkMonitorStorage,

    /// How often the list of bonded nym-nodes is refreshed from the mixnet contract
    /// (e.g. `10m`, `1h`).
    pub(crate) node_refresh_rate: Duration,

    /// Timeout for querying a single node for its detailed information (sphinx key, noise key,
    /// etc.). A node that exceeds this budget keeps whatever an earlier cycle learned about it
    /// (e.g. `10s`).
    pub(crate) node_info_query_timeout: Duration,

    /// Maximum number of nodes queried concurrently during a node refresh cycle.
    pub(crate) number_of_concurrent_node_queries: usize,

    pub(crate) shutdown_token: ShutdownToken,
}

/// What one node's refresh produced. The two cases are persisted differently, and keeping them
/// apart in the type is what makes "described completely or not at all" checkable rather than a
/// convention: there is no value of this type that carries a half-described node.
enum RefreshedNode {
    /// Everything the node's own endpoint reported, all from one reading of it.
    Described(NewNymNode),

    /// The node is bonded, but its endpoint did not answer (or answered incompletely), so only that
    /// much is known this cycle.
    BondOnly(BondedNymNode),
}

impl RefreshedNode {
    fn described(self) -> Option<NewNymNode> {
        match self {
            RefreshedNode::Described(node) => Some(node),
            RefreshedNode::BondOnly(_) => None,
        }
    }

    fn bond_only(self) -> Option<BondedNymNode> {
        match self {
            RefreshedNode::BondOnly(node) => Some(node),
            RefreshedNode::Described(_) => None,
        }
    }
}

/// Information about the node retrieved from the node directly
struct SelfDescribedData {
    /// Mixnet socket address (host:port) at which the node accepts sphinx packets.
    mixnet_socket_address: SocketAddr,

    /// Every ip address announced by the node, canonicalised, deduplicated and sorted.
    /// Test runs rotate through this set, which is why the order has to be stable across
    /// refreshes rather than however the node happened to report it.
    announced_ips: Vec<IpAddr>,

    /// X25519 public key used for Noise handshakes
    noise_key: x25519::PublicKey,

    /// Sphinx public key used for packet encryption
    sphinx_key: x25519::PublicKey,

    /// Key rotation epoch ID that `sphinx_key` belongs to.
    key_rotation_id: KeyRotationId,

    /// The supported roles of the node in the network.
    roles: NodeRoles,

    /// Port of the node's PLAIN client websocket listener, which a gateway liveness probe opens its
    /// session on. `None` for a node announcing no entry-gateway interface, and for one whose
    /// websocket query failed.
    ///
    /// Its `wss` counterpart is deliberately not read: nothing anywhere in the subsystem consumes
    /// it. The probe targets `ws://<ip>` by construction, no submission carries the fact, and the
    /// divergence surface in nym-api does not bucket on it.
    clients_ws_port: Option<u16>,
}

impl NodeRefresher {
    pub(crate) fn new(
        config: &Config,
        client: QueryHttpRpcNyxdClient,
        storage: NetworkMonitorStorage,
        shutdown_token: ShutdownToken,
    ) -> Self {
        NodeRefresher {
            client,
            storage,
            node_refresh_rate: config.node_refresh_rate,
            node_info_query_timeout: config.node_info_query_timeout,
            number_of_concurrent_node_queries: config.number_of_concurrent_node_queries,
            shutdown_token,
        }
    }
    async fn get_node_details_inner(&self, bond: NymNodeBond) -> anyhow::Result<SelfDescribedData> {
        let node_id = bond.node_id;

        let client = NymNodeApiClientRetriever::new(bin_info!())
            .with_expected_identity(Some(bond.node.identity_key))
            .with_verify_host_information()
            .with_custom_port(bond.node.custom_http_port)
            .get_client(&bond.node.host, node_id)
            .await?;

        let api_client = client.client;
        let host_info = client
            .host_information
            .context("failed to query node host information")?;

        // retrieve information on the announced ports in case a non-custom mixnet port
        // is being used
        let aux = api_client.get_auxiliary_details().await?;

        // if the noise key is missing, it means the node is outdated,
        // so it does not support stress testing anyway
        let noise_key = host_info
            .keys
            .x25519_versioned_noise
            .context("missing noise key")?
            .x25519_pubkey;
        let sphinx_key = host_info.keys.primary_x25519_sphinx_key.public_key;
        let key_rotation_id = host_info.keys.primary_x25519_sphinx_key.rotation_id;

        // canonicalise, deduplicate and sort so that the rotation testruns perform over this set
        // is stable across refreshes - a node is free to report its addresses in any order, and a
        // resolved hostname may well report them in a different one every time
        let mut announced_ips = host_info
            .ip_address
            .iter()
            .map(|ip| ip.to_canonical())
            .collect::<Vec<_>>();
        announced_ips.sort_unstable();
        announced_ips.dedup();

        let ip_address = announced_ips
            .first()
            .context("node hasn't announced any IPs")?;
        let mix_port = aux
            .announce_ports
            .mix_port
            .unwrap_or(DEFAULT_MIX_LISTENING_PORT);

        // retrieve information about the node roles so that we can classify the node, and so that we
        // know whether to ask it about its client websocket interface at all
        let roles = api_client
            .get_roles()
            .await
            .context("failed to retrieve node roles")?;

        // the gateway liveness probe opens a client session, which needs the port that interface
        // listens on. asked for separately because it is not one of the announced ports, and only of
        // gateway-capable nodes, since a pure mixnode serves no client websocket. a gateway that
        // will not answer for it fails the whole describe rather than yielding a node described
        // everywhere except here
        let clients_ws_port = if roles.gateway_enabled {
            Some(
                api_client
                    .get_mixnet_websockets()
                    .await
                    .context("failed to retrieve the client websocket interface")?
                    .ws_port,
            )
        } else {
            None
        };

        Ok(SelfDescribedData {
            // only contributes the mix port now that the address under test is picked per run
            mixnet_socket_address: SocketAddr::new(*ip_address, mix_port),
            announced_ips,
            noise_key,
            sphinx_key,
            key_rotation_id,
            roles,
            clients_ws_port,
        })
    }

    /// Refreshes one node, either completely or not at all.
    ///
    /// A node is described as a whole: every field comes from the same reading of its endpoint, so a
    /// row can never hold a fresh key beside an address from an earlier cycle. When any part of the
    /// describe fails, the outcome carries the bond alone and the node's previously learned fields
    /// are left exactly as they were, rather than being overwritten with nulls that would make an
    /// otherwise testable node ineligible for every kind until the next successful cycle.
    async fn get_node_details(&self, bond: NymNodeBond, timeout: Duration) -> RefreshedNode {
        let bonded = BondedNymNode::from_bond(&bond);

        let node_id = bond.node_id;
        let self_described = match tokio::time::timeout(timeout, self.get_node_details_inner(bond))
            .await
        {
            Err(_timeout) => {
                debug!(
                    "timed out while attempting to retrieve self-described node details for node {node_id}"
                );
                return RefreshedNode::BondOnly(bonded);
            }
            Ok(Err(err)) => {
                debug!("failed to retrieve self-described node details for node {node_id}: {err}");
                return RefreshedNode::BondOnly(bonded);
            }
            Ok(Ok(info)) => info,
        };

        RefreshedNode::Described(NewNymNode {
            node_id: bonded.node_id,
            identity_key: bonded.identity_key,
            last_seen_bonded: bonded.last_seen_bonded,
            // only contributes the mix port now that the address under test is picked per run
            mixnet_socket_address: Some(self_described.mixnet_socket_address.to_string()),
            announced_ips: Some(
                self_described
                    .announced_ips
                    .iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            noise_key: Some(self_described.noise_key.to_base58_string()),
            sphinx_key: Some(self_described.sphinx_key.to_base58_string()),
            key_rotation_id: Some(self_described.key_rotation_id as i64),
            node_type: NodeType::from_roles(&self_described.roles),
            clients_ws_port: self_described.clients_ws_port.map(i64::from),
        })
    }

    async fn refresh_bonded_nodes(&self) -> anyhow::Result<()> {
        let start = Instant::now();

        // 1. retrieve all nodes from the contract
        let nodes = self.client.get_all_nymnode_bonds().await?;
        let num_nodes = nodes.len();
        info!("retrieved {num_nodes} bonded nodes from the contract");

        // 2. retrieve detailed information from the self-described endpoints
        let timeout = self.node_info_query_timeout;
        let refreshed_nodes: Vec<_> = stream::iter(nodes)
            .map(|b| self.get_node_details(b, timeout))
            .buffer_unordered(self.number_of_concurrent_node_queries)
            .collect()
            .await;

        // the two outcomes are persisted differently: a described node replaces everything stored
        // about it, while one that could not be described only proves it is still bonded
        let (described, bond_only): (Vec<_>, Vec<_>) = refreshed_nodes
            .into_iter()
            .partition(|node| matches!(node, RefreshedNode::Described(_)));
        let described: Vec<_> = described
            .into_iter()
            .filter_map(RefreshedNode::described)
            .collect();
        let bond_only: Vec<_> = bond_only
            .into_iter()
            .filter_map(RefreshedNode::bond_only)
            .collect();

        let mut per_type: HashMap<NodeType, i64> = HashMap::new();
        for node in &described {
            *per_type.entry(node.node_type).or_insert(0) += 1;
        }
        let count_of = |t: NodeType| per_type.get(&t).copied().unwrap_or(0);
        // a described node reporting no roles at all is as unusable as one that never answered, so
        // both land in the unknown bucket
        let unknown = count_of(NodeType::Unknown) + bond_only.len() as i64;
        let successful = described.len() as i64 - count_of(NodeType::Unknown);
        info!("managed to retrieve full node information on {successful} nodes ({unknown} failed)");

        PROMETHEUS_METRICS.set(
            PrometheusMetric::BondedMixnodeNymNodes,
            count_of(NodeType::Mixnode),
        );
        PROMETHEUS_METRICS.set(
            PrometheusMetric::BondedGatewayNymNodes,
            count_of(NodeType::Gateway),
        );
        PROMETHEUS_METRICS.set(
            PrometheusMetric::BondedMixnodeAndGatewayNymNodes,
            count_of(NodeType::MixnodeAndGateway),
        );
        PROMETHEUS_METRICS.set(PrometheusMetric::BondedUnknownNymNodes, unknown);
        PROMETHEUS_METRICS.set(PrometheusMetric::SuccessfulNymNodeDataRetrieval, successful);
        PROMETHEUS_METRICS.set(PrometheusMetric::FailedNymNodeDataRetrieval, unknown);

        // 3. persist what each node yielded. A described node has every field replaced; one that
        //    could not be described has only its bond recorded, keeping whatever was learned about
        //    it before, since nulling that would drop an otherwise testable node out of every kind
        //    until a later cycle answered.
        self.storage
            .batch_insert_or_update_nym_nodes(&described)
            .await?;
        self.storage.batch_touch_bonded_nodes(&bond_only).await?;

        // Observe the cycle duration last so it reflects the full refresh path
        // (contract query + per-node queries + storage write).
        PROMETHEUS_METRICS.observe_histogram(
            PrometheusMetric::NodeRefreshCycleSeconds,
            start.elapsed().as_secs_f64(),
        );
        Ok(())
    }

    pub(crate) async fn run(&self) {
        let mut interval = interval(self.node_refresh_rate);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    break
                }
                _ = interval.tick() => {
                    if let Err(err) = self.refresh_bonded_nodes().await {
                        error!("failed to refresh bonded nodes: {err}");
                    }
                }
            }
        }

        info!("node refresher stopped");
    }
}
