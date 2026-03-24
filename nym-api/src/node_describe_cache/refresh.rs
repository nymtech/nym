// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::query_helpers::query_for_described_data;
use crate::node_describe_cache::NodeDescribeCacheError;
use nym_api_requests::models::{DescribedNodeTypeV2, NymNodeDescriptionV2};
use nym_bin_common::bin_info;
use nym_config::defaults::DEFAULT_NYM_NODE_HTTP_PORT;
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::{NodeId, NymNodeDetails};
use nym_node_requests::api::client::NymNodeApiClientExt;
use nym_node_requests::api::helpers::NymNodeApiClientRetriever;
use nym_node_requests::try_get_valid_nym_node_api_client;
use nym_validator_client::UserAgent;
use std::time::Duration;
use tracing::{debug, error};

#[derive(Debug)]
pub(crate) struct RefreshData {
    host: String,
    node_id: NodeId,
    expected_identity: ed25519::PublicKey,
    node_type: DescribedNodeTypeV2,

    port: Option<u16>,
}

impl<'a> TryFrom<&'a NymNodeDetails> for RefreshData {
    type Error = ed25519::Ed25519RecoveryError;

    fn try_from(node: &'a NymNodeDetails) -> Result<Self, Self::Error> {
        Ok(RefreshData::new(
            &node.bond_information.node.host,
            node.bond_information.identity().parse()?,
            DescribedNodeTypeV2::NymNode,
            node.node_id(),
            node.bond_information.node.custom_http_port,
        ))
    }
}

impl RefreshData {
    pub fn new(
        host: impl Into<String>,
        expected_identity: ed25519::PublicKey,
        node_type: DescribedNodeTypeV2,
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

    pub(crate) async fn try_refresh(self, allow_all_ips: bool) -> Option<NymNodeDescriptionV2> {
        match try_get_description(self, allow_all_ips).await {
            Ok(description) => Some(description),
            Err(err) => {
                debug!("failed to obtain node self-described data: {err}");
                None
            }
        }
    }
}

async fn try_get_description(
    data: RefreshData,
    allow_all_ips: bool,
) -> Result<NymNodeDescriptionV2, NodeDescribeCacheError> {
    let client = NymNodeApiClientRetriever::new(bin_info!())
        .with_expected_identity(Some(data.expected_identity.to_base58_string()))
        .with_verify_host_information()
        .with_custom_port(data.port)
        .get_client(&data.host, data.node_id)
        .await?;

    let host_info = match client.host_information {
        Some(host_info) => host_info,
        // this branch should be impossible unless unexpected code changes occurred
        None => {
            error!(
                "failed to retrieve host information of node {} - this is most likely a bug",
                data.node_id
            );
            return Err(NodeDescribeCacheError::NoHostInformationAvailable {
                node_id: data.node_id,
                host: data.host.to_string(),
            });
        }
    };

    if !allow_all_ips && !host_info.data.check_ips() {
        return Err(NodeDescribeCacheError::IllegalIpAddress {
            node_id: data.node_id,
        });
    }

    let node_info = query_for_described_data(&client.client, data.node_id).await?;
    let description = node_info.into_node_description(host_info.data);

    Ok(NymNodeDescriptionV2 {
        node_id: data.node_id,
        contract_node_type: data.node_type,
        description,
    })
}
