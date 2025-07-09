// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::query_helpers::query_for_described_data;
use crate::node_describe_cache::NodeDescribeCacheError;
use nym_api_requests::legacy::{LegacyGatewayBondWithId, LegacyMixNodeDetailsWithLayer};
use nym_api_requests::models::{DescribedNodeType, NymNodeDescription};
use nym_bin_common::bin_info;
use nym_config::defaults::DEFAULT_NYM_NODE_HTTP_PORT;
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::{NodeId, NymNodeDetails};
use nym_node_requests::api::client::NymNodeApiClientExt;
use nym_validator_client::UserAgent;
use std::time::Duration;
use tracing::debug;

#[derive(Debug)]
pub(crate) struct RefreshData {
    host: String,
    node_id: NodeId,
    expected_identity: ed25519::PublicKey,
    node_type: DescribedNodeType,

    port: Option<u16>,
}

impl<'a> TryFrom<&'a LegacyMixNodeDetailsWithLayer> for RefreshData {
    type Error = ed25519::Ed25519RecoveryError;

    fn try_from(node: &'a LegacyMixNodeDetailsWithLayer) -> Result<Self, Self::Error> {
        Ok(RefreshData::new(
            &node.bond_information.mix_node.host,
            node.bond_information.identity().parse()?,
            DescribedNodeType::LegacyMixnode,
            node.mix_id(),
            Some(node.bond_information.mix_node.http_api_port),
        ))
    }
}

impl<'a> TryFrom<&'a LegacyGatewayBondWithId> for RefreshData {
    type Error = ed25519::Ed25519RecoveryError;

    fn try_from(node: &'a LegacyGatewayBondWithId) -> Result<Self, Self::Error> {
        Ok(RefreshData::new(
            &node.bond.gateway.host,
            node.bond.identity().parse()?,
            DescribedNodeType::LegacyGateway,
            node.node_id,
            None,
        ))
    }
}

impl<'a> TryFrom<&'a NymNodeDetails> for RefreshData {
    type Error = ed25519::Ed25519RecoveryError;

    fn try_from(node: &'a NymNodeDetails) -> Result<Self, Self::Error> {
        Ok(RefreshData::new(
            &node.bond_information.node.host,
            node.bond_information.identity().parse()?,
            DescribedNodeType::NymNode,
            node.node_id(),
            node.bond_information.node.custom_http_port,
        ))
    }
}

impl RefreshData {
    pub fn new(
        host: impl Into<String>,
        expected_identity: ed25519::PublicKey,
        node_type: DescribedNodeType,
        node_id: NodeId,
        port: Option<u16>,
    ) -> Self {
        RefreshData {
            host: host.into(),
            node_id,
            expected_identity,
            node_type,
            port,
        }
    }

    pub(crate) fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub(crate) async fn try_refresh(self, allow_all_ips: bool) -> Option<NymNodeDescription> {
        match try_get_description(self, allow_all_ips).await {
            Ok(description) => Some(description),
            Err(err) => {
                debug!("failed to obtain node self-described data: {err}");
                None
            }
        }
    }
}

async fn try_get_client(
    host: &str,
    node_id: NodeId,
    custom_port: Option<u16>,
) -> Result<nym_node_requests::api::Client, NodeDescribeCacheError> {
    // first try the standard port in case the operator didn't put the node behind the proxy,
    // then default https (443)
    // finally default http (80)
    let mut addresses_to_try = vec![
        format!("http://{host}:{DEFAULT_NYM_NODE_HTTP_PORT}"), // 'standard' nym-node
        format!("https://{host}"),                             // node behind https proxy (443)
        format!("http://{host}"),                              // node behind http proxy (80)
    ];

    // note: I removed 'standard' legacy mixnode port because it should now be automatically pulled via
    // the 'custom_port' since it should have been present in the contract.

    if let Some(port) = custom_port {
        addresses_to_try.insert(0, format!("http://{host}:{port}"));
    }

    for address in addresses_to_try {
        // if provided host was malformed, no point in continuing
        let client = match nym_node_requests::api::Client::builder(address).and_then(|b| {
            b.with_timeout(Duration::from_secs(5))
                .no_hickory_dns()
                .with_user_agent(UserAgent::from(bin_info!()))
                .build()
        }) {
            Ok(client) => client,
            Err(err) => {
                return Err(NodeDescribeCacheError::MalformedHost {
                    host: host.to_string(),
                    node_id,
                    source: err,
                });
            }
        };

        if let Ok(health) = client.get_health().await {
            if health.status.is_up() {
                return Ok(client);
            }
        }
    }

    Err(NodeDescribeCacheError::NoHttpPortsAvailable {
        host: host.to_string(),
        node_id,
    })
}

async fn try_get_description(
    data: RefreshData,
    allow_all_ips: bool,
) -> Result<NymNodeDescription, NodeDescribeCacheError> {
    let client = try_get_client(&data.host, data.node_id, data.port).await?;

    let map_query_err = |err| NodeDescribeCacheError::ApiFailure {
        node_id: data.node_id,
        source: err,
    };

    let host_info = client.get_host_information().await.map_err(map_query_err)?;

    // check if the identity key matches the information provided during bonding
    if data.expected_identity != host_info.keys.ed25519_identity {
        return Err(NodeDescribeCacheError::MismatchedIdentity {
            node_id: data.node_id,
            expected: data.expected_identity.to_base58_string(),
            got: host_info.keys.ed25519_identity.to_base58_string(),
        });
    }

    if !host_info.verify_host_information() {
        return Err(NodeDescribeCacheError::MissignedHostInformation {
            node_id: data.node_id,
        });
    }

    if !allow_all_ips && !host_info.data.check_ips() {
        return Err(NodeDescribeCacheError::IllegalIpAddress {
            node_id: data.node_id,
        });
    }

    let node_info = query_for_described_data(&client, data.node_id).await?;
    let description = node_info.into_node_description(host_info.data);

    Ok(NymNodeDescription {
        node_id: data.node_id,
        contract_node_type: data.node_type,
        description,
    })
}
