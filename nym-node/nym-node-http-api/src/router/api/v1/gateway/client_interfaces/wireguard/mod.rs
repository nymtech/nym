// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::api::v1::gateway::client_interfaces::wireguard::client_registry::{
    get_all_clients, get_client, register_client,
};
use crate::error::NymNodeHttpError;
use axum::routing::{get, post};
use axum::Router;
use ipnetwork::IpNetwork;
use nym_crypto::asymmetric::encryption::PrivateKey;
use nym_node_requests::routes::api::v1::gateway::client_interfaces::wireguard;
use nym_wireguard::setup;
use nym_wireguard_types::registration::PrivateIPs;
use nym_wireguard_types::registration::{GatewayClientRegistry, PendingRegistrations};
use std::sync::Arc;

pub(crate) mod client_registry;
mod error;

// I don't see any reason why this state should be accessible to any routes outside /wireguard
// if anyone finds compelling reason, it could be moved to the `AppState` struct instead
#[derive(Clone, Default)]
pub struct WireguardAppState {
    inner: Option<WireguardAppStateInner>,
}

impl WireguardAppState {
    pub fn new(
        client_registry: Arc<GatewayClientRegistry>,
        registration_in_progress: Arc<PendingRegistrations>,
        binding_port: u16,
        private_ip_network: IpNetwork,
    ) -> Result<Self, NymNodeHttpError> {
        Ok(WireguardAppState {
            inner: Some(WireguardAppStateInner {
                private_key: Arc::new(PrivateKey::from_bytes(
                    setup::server_static_private_key().as_ref(),
                )?),
                client_registry,
                registration_in_progress,
                binding_port,
                free_private_network_ips: Arc::new(
                    private_ip_network.iter().map(|ip| (ip, true)).collect(),
                ),
            }),
        })
    }

    // #[allow(dead_code)]
    // pub(crate) fn dh_keypair(&self) -> Option<&encryption::KeyPair> {
    //     self.inner.as_ref().map(|s| s.dh_keypair.as_ref())
    // }
    //
    // #[allow(dead_code)]
    // pub(crate) fn client_registry(&self) -> Option<&RwLock<ClientRegistry>> {
    //     self.inner.as_ref().map(|s| s.client_registry.as_ref())
    // }
    //
    // #[allow(dead_code)]
    // pub(crate) fn registration_in_progress(&self) -> Option<&RwLock<PendingRegistrations>> {
    //     self.inner
    //         .as_ref()
    //         .map(|s| s.registration_in_progress.as_ref())
    // }

    // not sure what to feel about exposing this method
    pub(crate) fn inner(&self) -> Option<&WireguardAppStateInner> {
        self.inner.as_ref()
    }
}

// helper macro to deal with missing wg state (if not being exposed by the node)
#[macro_export]
macro_rules! get_state {
    ( $state: ident, $field: ident ) => {{
        let Some(ref inner) = $state.inner else {
            return ::axum::http::StatusCode::NOT_IMPLEMENTED;
        };
        inner.$field.as_ref()
    }};
}

#[derive(Clone)]
pub(crate) struct WireguardAppStateInner {
    private_key: Arc<PrivateKey>,
    client_registry: Arc<GatewayClientRegistry>,
    registration_in_progress: Arc<PendingRegistrations>,
    binding_port: u16,
    free_private_network_ips: Arc<PrivateIPs>,
}

pub(crate) fn routes<S>(initial_state: WireguardAppState) -> Router<S> {
    Router::new()
        // .route("/", get())
        .route(wireguard::CLIENTS, get(get_all_clients))
        .route(wireguard::CLIENT, post(register_client))
        .route(&format!("{}/:pub_key", wireguard::CLIENT), get(get_client))
        .with_state(initial_state)
}

#[cfg(test)]
mod test {
    use crate::api::v1::gateway::client_interfaces::wireguard::{
        routes, WireguardAppState, WireguardAppStateInner,
    };
    use axum::body::Body;
    use axum::http::Request;
    use axum::http::StatusCode;
    use dashmap::DashMap;
    use hmac::Mac;
    use ipnetwork::IpNetwork;
    use nym_crypto::asymmetric::encryption;
    use nym_node_requests::api::v1::gateway::client_interfaces::wireguard::models::{
        ClientMac, ClientMessage, ClientRegistrationResponse, GatewayClient, InitMessage,
        PeerPublicKey,
    };
    use nym_node_requests::routes::api::v1::gateway::client_interfaces::wireguard;
    use nym_wireguard::setup::server_static_private_key;
    use nym_wireguard_types::registration::HmacSha256;
    use std::net::IpAddr;
    use std::str::FromStr;
    use std::sync::Arc;
    use tower::Service;
    use tower::ServiceExt;
    use x25519_dalek::{PublicKey, StaticSecret};

    #[tokio::test]
    async fn registration() {
        // 1. Provision random keys for gateway and client
        // 2. Generate DH shared secret
        // 3. Client submits its public key to the gateway to start the handshake process, gateway responds with nonce
        // 4. Client generates mac digest using DH shared secret, its own public key, socket address and port, and nonce
        // 5. Client sends its public key, socket address and port, nonce and mac digest to the gateway
        // 6. Gateway verifies mac digest and nonce, and stores client's public key and socket address and port

        let mut rng = rand::thread_rng();
        let gateway_private_key =
            encryption::PrivateKey::from_bytes(server_static_private_key().as_bytes()).unwrap();
        let gateway_public_key = encryption::PublicKey::from(&gateway_private_key);

        let gateway_key_pair = encryption::KeyPair::from_bytes(
            &gateway_private_key.to_bytes(),
            &gateway_public_key.to_bytes(),
        )
        .unwrap();
        let client_key_pair = encryption::KeyPair::new(&mut rng);

        let gateway_static_public = PublicKey::from(gateway_key_pair.public_key().to_bytes());

        let client_static_private = StaticSecret::from(client_key_pair.private_key().to_bytes());
        let client_static_public = PublicKey::from(client_key_pair.public_key().to_bytes());

        let client_dh = client_static_private.diffie_hellman(&gateway_static_public);

        let registration_in_progress = Arc::new(DashMap::new());
        let client_registry = Arc::new(DashMap::new());
        let free_private_network_ips = Arc::new(
            IpNetwork::from_str("10.1.0.0/24")
                .unwrap()
                .iter()
                .map(|ip| (ip, true))
                .collect(),
        );
        let client_private_ip = IpAddr::from_str("10.1.0.42").unwrap();

        let state = WireguardAppState {
            inner: Some(WireguardAppStateInner {
                client_registry: Arc::clone(&client_registry),
                private_key: Arc::new(gateway_private_key),
                registration_in_progress: Arc::clone(&registration_in_progress),
                binding_port: 8080,
                free_private_network_ips,
            }),
        };

        // `Router` implements `tower::Service<Request<Body>>` so we can
        // call it like any tower service, no need to run an HTTP server.
        let mut app = routes(state);

        let init_message = ClientMessage::Initial(InitMessage {
            pub_key: PeerPublicKey::new(client_static_public),
        });

        let init_request = Request::builder()
            .method("POST")
            .uri(wireguard::CLIENT)
            .header("Content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&init_message).unwrap()))
            .unwrap();

        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(init_request)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(!registration_in_progress.is_empty());

        let ClientRegistrationResponse::PendingRegistration {
            nonce,
            gateway_data,
            wg_port: 8080,
        } = serde_json::from_slice(&hyper::body::to_bytes(response.into_body()).await.unwrap())
            .unwrap()
        else {
            panic!("invalid response")
        };
        assert!(gateway_data
            .verify(client_key_pair.private_key(), nonce)
            .is_ok());

        let mut mac = HmacSha256::new_from_slice(client_dh.as_bytes()).unwrap();
        mac.update(client_static_public.as_bytes());
        mac.update(client_private_ip.to_string().as_bytes());
        mac.update(&nonce.to_le_bytes());
        let mac = mac.finalize().into_bytes();

        let finalized_message = ClientMessage::Final(GatewayClient {
            pub_key: PeerPublicKey::new(client_static_public),
            private_ip: client_private_ip,
            mac: ClientMac::new(mac.as_slice().to_vec()),
        });

        let final_request = Request::builder()
            .method("POST")
            .uri(wireguard::CLIENT)
            .header("Content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&finalized_message).unwrap()))
            .unwrap();

        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(final_request)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(!client_registry.is_empty());

        let clients_request = Request::builder()
            .method("GET")
            .uri(wireguard::CLIENTS)
            .body(Body::empty())
            .unwrap();

        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(clients_request)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let clients: Vec<PeerPublicKey> =
            serde_json::from_slice(&hyper::body::to_bytes(response.into_body()).await.unwrap())
                .unwrap();

        assert!(!clients.is_empty());

        assert_eq!(
            client_registry
                .iter()
                .map(|c| c.value().pub_key())
                .collect::<Vec<PeerPublicKey>>(),
            clients
        )
    }
}
