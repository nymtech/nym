// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::GatewayError;
use crate::helpers::load_public_key;
use nym_bin_common::bin_info_owned;
use nym_crypto::asymmetric::{encryption, identity};
use nym_network_requester::RequestFilter;
use nym_node_http_api::api::api_requests;
use nym_node_http_api::api::api_requests::v1::network_requester::exit_policy::models::UsedExitPolicy;
use nym_node_http_api::api::api_requests::SignedHostInformation;
use nym_node_http_api::NymNodeHttpError;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::TaskClient;
use std::sync::Arc;
use tracing::{debug, error, warn};

fn load_gateway_details(
    config: &Config,
) -> Result<api_requests::v1::gateway::models::Gateway, GatewayError> {
    Ok(api_requests::v1::gateway::models::Gateway {
        enforces_zk_nyms: config.gateway.only_coconut_credentials,
        client_interfaces: api_requests::v1::gateway::models::ClientInterfaces {
            wireguard: None,
            mixnet_websockets: Some(api_requests::v1::gateway::models::WebSockets {
                ws_port: config.gateway.clients_port,
                wss_port: config.gateway.clients_wss_port,
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
        ip_address: config.host.public_ips.clone(),
        hostname: config.host.hostname.clone(),
        keys: api_requests::v1::node::models::HostKeys {
            ed25519_identity: identity_keypair.public_key().to_base58_string(),
            x25519_sphinx: sphinx_key.to_base58_string(),
            x25519_noise: "".to_string(),
        },
    };

    let signed_info = SignedHostInformation::new(host_info, identity_keypair.private_key())
        .map_err(NymNodeHttpError::from)?;
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

fn load_ip_packet_router_details(
    config: &Config,
    ip_packet_router_config: &nym_ip_packet_router::Config,
) -> Result<api_requests::v1::ip_packet_router::models::IpPacketRouter, GatewayError> {
    let identity_public_key: identity::PublicKey = load_public_key(
        &ip_packet_router_config
            .storage_paths
            .common_paths
            .keys
            .public_identity_key_file,
        "ip packet router identity",
    )?;

    let dh_public_key: encryption::PublicKey = load_public_key(
        &ip_packet_router_config
            .storage_paths
            .common_paths
            .keys
            .public_encryption_key_file,
        "ip packet router diffie hellman",
    )?;

    let gateway_identity_public_key: identity::PublicKey = load_public_key(
        &config.storage_paths.keys.public_identity_key_file,
        "gateway identity",
    )?;

    Ok(api_requests::v1::ip_packet_router::models::IpPacketRouter {
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

pub(crate) struct HttpApiBuilder<'a> {
    gateway_config: &'a Config,
    network_requester_config: Option<&'a nym_network_requester::Config>,
    exit_policy: Option<UsedExitPolicy>,
    ip_packet_router_config: Option<&'a nym_ip_packet_router::Config>,

    identity_keypair: &'a identity::KeyPair,
    // TODO: this should be a wg specific key and not re-used sphinx
    sphinx_keypair: Arc<encryption::KeyPair>,
}

impl<'a> HttpApiBuilder<'a> {
    pub(crate) fn new(
        gateway_config: &'a Config,
        identity_keypair: &'a identity::KeyPair,
        sphinx_keypair: Arc<encryption::KeyPair>,
    ) -> Self {
        HttpApiBuilder {
            gateway_config,
            network_requester_config: None,
            ip_packet_router_config: None,
            exit_policy: None,
            identity_keypair,
            sphinx_keypair,
        }
    }

    #[must_use]
    pub(crate) fn with_maybe_network_requester(
        mut self,
        network_requester_config: Option<&'a nym_network_requester::Config>,
    ) -> Self {
        self.network_requester_config = network_requester_config;
        self
    }

    #[must_use]
    pub(crate) fn with_maybe_network_request_filter(
        mut self,
        request_filter: Option<RequestFilter>,
    ) -> Self {
        let Some(request_filter) = request_filter else {
            warn!("no valid request filter has been passed. no changes will be made");
            return self;
        };

        // we can cheat here a bit since we're not refreshing the exit policy
        // thus:
        // - we can ignore the Arc pointer and clone the inner value
        // - we can set the last refresh time to the current time
        //
        // once we start refreshing it, we'll have to change it, but at that point
        // the allow list will be probably be completely removed and thus the pointer management
        // will be much easier

        let upstream = request_filter.current_exit_policy_filter().upstream();

        // if there's no upstream (i.e. open proxy), we couldn't have possibly updated it : )
        let last_updated = if upstream.is_some() {
            #[allow(clippy::expect_used)]
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock is set to before the unix epoch")
                .as_secs()
        } else {
            0
        };

        self.exit_policy = Some(UsedExitPolicy {
            enabled: true,
            upstream_source: upstream.map(|u| u.to_string()).unwrap_or_default(),
            last_updated,
            policy: Some(request_filter.current_exit_policy_filter().policy().clone()),
        });

        self
    }

    #[must_use]
    pub(crate) fn with_maybe_ip_packet_router(
        mut self,
        ip_packet_router_config: Option<&'a nym_ip_packet_router::Config>,
    ) -> Self {
        self.ip_packet_router_config = ip_packet_router_config;
        self
    }

    pub(crate) fn start(self, task_client: TaskClient) -> Result<(), GatewayError> {
        debug!("starting http API");

        // is it suboptimal to load all the keys, etc for the second time after they've already been
        // retrieved during startup of the rest of the components?
        // yes, a bit.
        // but in the grand scheme of things performance penalty is negligible since it's only happening on startup
        // and makes the code a bit nicer to manage. on top of it, all of it will refactored anyway at some point
        // (famous last words, eh? - 22.09.23)
        let mut config = nym_node_http_api::Config::new(
            bin_info_owned!(),
            load_host_details(
                self.gateway_config,
                self.sphinx_keypair.public_key(),
                self.identity_keypair,
            )?,
        )
        .with_gateway(load_gateway_details(self.gateway_config)?)
        .with_landing_page_assets(self.gateway_config.http.landing_page_assets_path.as_ref());

        if let Some(nr_config) = self.network_requester_config {
            config = config.with_network_requester(load_network_requester_details(
                self.gateway_config,
                nr_config,
            )?);

            if let Some(exit_policy) = self.exit_policy {
                config = config.with_used_exit_policy(exit_policy)
            }
        }

        if let Some(ipr_config) = self.ip_packet_router_config {
            config = config.with_ip_packet_router(load_ip_packet_router_details(
                self.gateway_config,
                ipr_config,
            )?);
        }

        let bind_address = self.gateway_config.http.bind_address;
        let router = nym_node_http_api::NymNodeRouter::new(config, None);

        tokio::spawn(async move {
            let server = match router.build_server(&bind_address).await {
                Ok(server) => server.with_task_client(task_client),
                Err(err) => {
                    error!("failed to create http server: {err}");
                    return;
                }
            };
            server.run().await
        });
        Ok(())
    }
}

// pub(crate) fn start_http_api(
//     gateway_config: &Config,
//     network_requester_config: Option<&nym_network_requester::Config>,
//     client_registry: Arc<GatewayClientRegistry>,
//     identity_keypair: &identity::KeyPair,
//     // TODO: this should be a wg specific key and not re-used sphinx
//     sphinx_keypair: Arc<encryption::KeyPair>,
//
//     task_client: TaskClient,
// ) -> Result<(), GatewayError> {
//     HttpApiBuilder::new(gateway_config, identity_keypair, sphinx_keypair)
//         .with_wireguard_client_registry(client_registry)
//         .with_network_requester(network_requester_config)
//         .start(task_client)
// }
