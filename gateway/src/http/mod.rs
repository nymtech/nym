// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::error::GatewayError;
use crate::node::helpers::load_public_key;
use nym_bin_common::bin_info_owned;
use nym_crypto::asymmetric::{encryption, identity};
use nym_node::http;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::TaskClient;

fn load_gateway_details(
    config: &Config,
) -> Result<http::api::v1::gateway::types::Gateway, GatewayError> {
    Ok(http::api::v1::gateway::types::Gateway {
        client_interfaces: http::api::v1::gateway::types::ClientInterfaces {
            wireguard: None,
            mixnet_websockets: Some(http::api::v1::gateway::types::WebSockets {
                ws_port: config.gateway.clients_port,
                wss_port: None,
            }),
        },
    })
}

fn load_host_details(
    config: &Config,
) -> Result<http::api::v1::node::types::HostInformation, GatewayError> {
    let identity_public_key: identity::PublicKey = load_public_key(
        &config.storage_paths.keys.public_identity_key_file,
        "gateway identity",
    )?;

    let sphinx_public_key: encryption::PublicKey = load_public_key(
        &config.storage_paths.keys.public_sphinx_key_file,
        "gateway sphinx",
    )?;

    Ok(http::api::v1::node::types::HostInformation {
        // TODO: this should be extracted differently, i.e. it's the issue of the public/private address
        ip_address: vec![config.gateway.listening_address.to_string()],
        hostname: None,
        keys: http::api::v1::node::types::HostKeys {
            ed25519: identity_public_key.to_base58_string(),
            x25519: sphinx_public_key.to_base58_string(),
        },
    })
}

fn load_network_requester_details(
    config: &Config,
    network_requester_config: &nym_network_requester::Config,
) -> Result<http::api::v1::network_requester::types::NetworkRequester, GatewayError> {
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

    Ok(http::api::v1::network_requester::types::NetworkRequester {
        encoded_identity_key: identity_public_key.to_base58_string(),
        encoded_x25519_key: dh_public_key.to_base58_string(),
        address: Recipient::new(
            identity_public_key,
            dh_public_key,
            gateway_identity_public_key,
        )
        .to_string(),
    })
}

pub(crate) fn start_http_api(
    gateway_config: &Config,
    network_requester_config: Option<&nym_network_requester::Config>,
    task_client: TaskClient,
) -> Result<(), GatewayError> {
    // is it suboptimal to load all the keys, etc for the second time after they've already been
    // retrieved during startup of the rest of the components?
    // yes, a bit.
    // but in the grand scheme of things performance penalty is negligible since it's only happening on startup
    // and makes the code a bit nicer to manage. on top of it, all of it will refactored anyway at some point
    // (famous last words, eh? - 22.09.23)

    // TODO: load private key, set zeroizing wrapper and sign whatever responses we need to sign
    let mut config =
        nym_node::http::Config::new(bin_info_owned!(), load_host_details(gateway_config)?)
            .with_gateway(load_gateway_details(gateway_config)?);

    if let Some(nr_config) = network_requester_config {
        config = config
            .with_network_requester(load_network_requester_details(gateway_config, nr_config)?)
    }

    let router = nym_node::http::NymNodeRouter::new(config);

    let server = router
        .build_server(&gateway_config.http.bind_address)?
        .with_task_client(task_client);
    tokio::spawn(async move { server.run().await });
    Ok(())
}
