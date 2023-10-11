// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::error::GatewayError;
use crate::node::helpers::load_public_key;
use nym_bin_common::bin_info_owned;
use nym_crypto::asymmetric::{encryption, identity};
use nym_node::error::NymNodeError;
use nym_node::http::api::api_requests;
use nym_node::http::api::api_requests::SignedHostInformation;
use nym_node::http::router::WireguardAppState;
use nym_node::wireguard::types::ClientRegistry;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::TaskClient;
use std::sync::Arc;
use tokio::sync::RwLock;

fn load_gateway_details(
    config: &Config,
) -> Result<api_requests::v1::gateway::models::Gateway, GatewayError> {
    Ok(api_requests::v1::gateway::models::Gateway {
        client_interfaces: api_requests::v1::gateway::models::ClientInterfaces {
            wireguard: None,
            mixnet_websockets: Some(api_requests::v1::gateway::models::WebSockets {
                ws_port: config.gateway.clients_port,
                wss_port: None,
            }),
        },
    })
}

fn load_host_details(
    config: &Config,
    sphinx_key: &encryption::PublicKey,
    identity_keypair: &identity::KeyPair,
) -> Result<api_requests::v1::node::models::SignedHostInformation, GatewayError> {
    let host_info = api_requests::v1::node::models::HostInformation {
        // TODO: this should be extracted differently, i.e. it's the issue of the public/private address
        ip_address: vec![config.gateway.listening_address.to_string()],
        hostname: None,
        keys: api_requests::v1::node::models::HostKeys {
            ed25519: identity_keypair.public_key().to_base58_string(),
            x25519: sphinx_key.to_base58_string(),
        },
    };

    let signed_info = SignedHostInformation::new(host_info, identity_keypair.private_key())
        .map_err(NymNodeError::from)?;
    Ok(signed_info)
}

fn load_network_requester_details(
    config: &Config,
    network_requester_config: &nym_network_requester::Config,
) -> Result<api_requests::v1::network_requester::models::NetworkRequester, GatewayError> {
    let identity_public_key: identity::PublicKey = load_public_key(
        &network_requester_config
            .storage_paths
            .common_paths
            .keys
            .public_identity_key_file,
        "network requester identity",
    )?;

    let dh_public_key: encryption::PublicKey = load_public_key(
        &network_requester_config
            .storage_paths
            .common_paths
            .keys
            .public_encryption_key_file,
        "network requester diffie hellman",
    )?;

    let gateway_identity_public_key: identity::PublicKey = load_public_key(
        &config.storage_paths.keys.public_identity_key_file,
        "gateway identity",
    )?;

    Ok(
        api_requests::v1::network_requester::models::NetworkRequester {
            encoded_identity_key: identity_public_key.to_base58_string(),
            encoded_x25519_key: dh_public_key.to_base58_string(),
            address: Recipient::new(
                identity_public_key,
                dh_public_key,
                gateway_identity_public_key,
            )
            .to_string(),
        },
    )
}

pub(crate) fn start_http_api(
    gateway_config: &Config,
    network_requester_config: Option<&nym_network_requester::Config>,
    client_registry: Arc<RwLock<ClientRegistry>>,
    identity_keypair: &identity::KeyPair,
    // TODO: this should be a wg specific key and not re-used sphinx
    sphinx_keypair: Arc<encryption::KeyPair>,

    task_client: TaskClient,
) -> Result<(), GatewayError> {
    // is it suboptimal to load all the keys, etc for the second time after they've already been
    // retrieved during startup of the rest of the components?
    // yes, a bit.
    // but in the grand scheme of things performance penalty is negligible since it's only happening on startup
    // and makes the code a bit nicer to manage. on top of it, all of it will refactored anyway at some point
    // (famous last words, eh? - 22.09.23)
    let mut config = nym_node::http::Config::new(
        bin_info_owned!(),
        load_host_details(
            gateway_config,
            sphinx_keypair.public_key(),
            identity_keypair,
        )?,
    )
    .with_gateway(load_gateway_details(gateway_config)?);

    if let Some(nr_config) = network_requester_config {
        config = config
            .with_network_requester(load_network_requester_details(gateway_config, nr_config)?)
    }

    let wg_state = WireguardAppState::new(sphinx_keypair, client_registry, Default::default());
    let router = nym_node::http::NymNodeRouter::new(config, Some(wg_state));

    let server = router
        .build_server(&gateway_config.http.bind_address)?
        .with_task_client(task_client);
    tokio::spawn(async move { server.run().await });
    Ok(())
}
