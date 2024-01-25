// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::NymNodeDescription;
use nym_config::defaults::DEFAULT_HTTP_API_LISTENING_PORT;
use nym_contracts_common::IdentityKey;
use nym_mixnet_contract_common::MixNode;
use nym_node_requests::api::client::NymNodeApiClientExt;

use super::NodeDescribeCacheError;

//this is a copy of try_get_client but for mixnode, to be deleted after smoosh probably
async fn try_get_client(
    mixnode: &MixNode,
) -> Result<nym_node_requests::api::Client, NodeDescribeCacheError> {
    let mixnode_host = &mixnode.host;

    // first try the standard port in case the operator didn't put the node behind the proxy,
    // then default https (443)
    // finally default http (80)
    let addresses_to_try = vec![
        format!("http://{mixnode_host}:{DEFAULT_HTTP_API_LISTENING_PORT}"),
        format!("https://{mixnode_host}"),
        format!("http://{mixnode_host}"),
    ];

    for address in addresses_to_try {
        // if provided host was malformed, no point in continuing
        let client = match nym_node_requests::api::Client::new_url(address, None) {
            Ok(client) => client,
            Err(err) => {
                return Err(NodeDescribeCacheError::MalformedHost {
                    host: mixnode_host.clone(),
                    gateway: mixnode.identity_key.clone(),
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
        host: mixnode_host.clone(),
        gateway: mixnode.identity_key.clone(),
    })
}

pub(crate) async fn get_mixnode_description(
    mixnode: MixNode,
) -> Result<(IdentityKey, NymNodeDescription), NodeDescribeCacheError> {
    let client = try_get_client(&mixnode).await?;

    let host_info =
        client
            .get_host_information()
            .await
            .map_err(|err| NodeDescribeCacheError::ApiFailure {
                gateway: mixnode.identity_key.clone(),
                source: err,
            })?;

    if !host_info.verify_host_information() {
        return Err(NodeDescribeCacheError::MissignedHostInformation {
            gateway: mixnode.identity_key,
        });
    }

    let build_info =
        client
            .get_build_information()
            .await
            .map_err(|err| NodeDescribeCacheError::ApiFailure {
                gateway: mixnode.identity_key.clone(),
                source: err,
            })?;

    //SW fill in
    // let noise = todo!();

    let description = NymNodeDescription {
        host_information: host_info.data,
        build_information: build_info,
        network_requester: None,
        ip_packet_router: None,
        mixnet_websockets: None,
    };

    Ok((mixnode.identity_key, description))
}
