// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::config::Config;
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::NewNymNode;
use anyhow::Context;
use futures::{StreamExt, stream};
use nym_bin_common::bin_info;
use nym_crypto::asymmetric::x25519;
use nym_network_defaults::DEFAULT_MIX_LISTENING_PORT;
use nym_node_requests::api::client::NymNodeApiClientExt;
use nym_node_requests::api::helpers::NymNodeApiClientRetriever;
use nym_task::ShutdownToken;
use nym_validator_client::QueryHttpRpcNyxdClient;
use nym_validator_client::models::KeyRotationId;
use nym_validator_client::nyxd::contract_traits::PagedMixnetQueryClient;
use nym_validator_client::nyxd::nym_mixnet_contract_common::NymNodeBond;
use rand::prelude::SliceRandom;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{error, info};

pub(crate) struct NodeRefresher {
    pub(crate) client: QueryHttpRpcNyxdClient,

    pub(crate) storage: NetworkMonitorStorage,

    /// How often the list of bonded nym-nodes is refreshed from the mixnet contract
    /// (e.g. `10m`, `1h`).
    pub(crate) node_refresh_rate: Duration,

    /// Timeout for querying a single node for its detailed information (sphinx key, noise key,
    /// etc.). Queries that exceed this budget leave the corresponding fields as `NULL`
    /// (e.g. `10s`).
    pub(crate) node_info_query_timeout: Duration,

    /// Maximum number of nodes queried concurrently during a node refresh cycle.
    pub(crate) number_of_concurrent_node_queries: usize,

    pub(crate) shutdown_token: ShutdownToken,
}

/// Information about the node retrieved from the node directly
struct SelfDescribedData {
    /// Mixnet socket address (host:port) at which the node accepts sphinx packets.
    mixnet_socket_address: SocketAddr,

    /// X25519 public key used for Noise handshakes
    noise_key: x25519::PublicKey,

    /// Sphinx public key used for packet encryption
    sphinx_key: x25519::PublicKey,

    /// Key rotation epoch ID that `sphinx_key` belongs to.
    key_rotation_id: KeyRotationId,
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

        // pseudorandomly choose which ip address to use - each announced address should work!
        let ip_address = host_info
            .ip_address
            .choose(&mut rand::thread_rng())
            .context("node hasn't announced any IPs")?;
        let mix_port = aux
            .announce_ports
            .mix_port
            .unwrap_or(DEFAULT_MIX_LISTENING_PORT);

        Ok(SelfDescribedData {
            mixnet_socket_address: SocketAddr::new(*ip_address, mix_port),
            noise_key,
            sphinx_key,
            key_rotation_id,
        })
    }

    async fn get_node_details(&self, bond: NymNodeBond) -> NewNymNode {
        let mut node_update = NewNymNode::from_bond(&bond);

        let node_id = bond.node_id;
        let self_described = match self.get_node_details_inner(bond).await {
            Err(err) => {
                error!("failed to retrieve self-described node details for node {node_id}: {err}");
                return node_update;
            }
            Ok(info) => info,
        };

        node_update.mixnet_socket_address = Some(self_described.mixnet_socket_address.to_string());
        node_update.noise_key = Some(self_described.noise_key.to_base58_string());
        node_update.sphinx_key = Some(self_described.sphinx_key.to_base58_string());
        node_update.key_rotation_id = Some(self_described.key_rotation_id as i64);

        node_update
    }

    async fn refresh_bonded_nodes(&self) -> anyhow::Result<()> {
        // 1. retrieve all nodes from the contract
        let nodes = self.client.get_all_nymnode_bonds().await?;
        info!("retrieved {} bonded nodes from the contract", nodes.len());

        // 2. retrieve detailed information from the self-described endpoints
        let refreshed_nodes = Mutex::new(Vec::new());
        stream::iter(nodes)
            .for_each_concurrent(self.number_of_concurrent_node_queries, |bond| async {
                let details = self.get_node_details(bond).await;
                refreshed_nodes.lock().await.push(details);
            })
            .await;

        let refreshed_nodes = refreshed_nodes.into_inner();
        info!(
            "managed to retrieve full node information on {} nodes",
            refreshed_nodes.len()
        );

        // 3. update the storage
        self.storage
            .batch_insert_or_update_nym_nodes(&refreshed_nodes)
            .await?;
        Ok(())
    }

    pub(crate) async fn run(&self) {
        let mut interval = interval(self.node_refresh_rate);

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
