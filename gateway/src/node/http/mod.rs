use std::{collections::HashMap, sync::Arc};

use axum::{
    routing::{get, post},
    Router,
};
use log::info;
use nym_crypto::asymmetric::encryption;
use tokio::sync::RwLock;

mod api;
use api::v1::client_registry::*;

use super::client_handling::client_registration::{ClientPublicKey, ClientRegistry};

const ROUTE_PREFIX: &str = "/api/v1/gateway/client-interfaces/wireguard";

pub struct ApiState {
    client_registry: Arc<RwLock<ClientRegistry>>,
    sphinx_key_pair: Arc<encryption::KeyPair>,
    registration_in_progress: Arc<RwLock<HashMap<ClientPublicKey, u64>>>,
}

fn router_with_state(state: Arc<ApiState>) -> Router {
    Router::new()
        .route(&format!("{}/clients", ROUTE_PREFIX), get(get_all_clients))
        .route(&format!("{}/client", ROUTE_PREFIX), post(register_client))
        .route(
            &format!("{}/client/:pub_key", ROUTE_PREFIX),
            get(get_client),
        )
        .with_state(state)
}

pub(crate) async fn start_http_api(
    client_registry: Arc<RwLock<ClientRegistry>>,
    sphinx_key_pair: Arc<encryption::KeyPair>,
) {
    // Port should be 80 post smoosh
    let port = 8000;

    info!("Started HTTP API on port {}", port);

    let client_registry = Arc::clone(&client_registry);

    let state = Arc::new(ApiState {
        client_registry,
        sphinx_key_pair,
        registration_in_progress: Arc::new(RwLock::new(HashMap::new())),
    });

    let routes = router_with_state(state);

    #[allow(clippy::unwrap_used)]
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(routes.into_make_service())
        .await
        .unwrap();
}

#[cfg(test)]
mod test {
    use std::net::SocketAddr;
    use std::str::FromStr;
    use std::{collections::HashMap, sync::Arc};

    use axum::body::Body;
    use axum::http::Request;
    use axum::http::StatusCode;
    use hmac::Mac;
    use tower::Service;
    use tower::ServiceExt;

    use nym_crypto::asymmetric::encryption;
    use tokio::sync::RwLock;
    use x25519_dalek::{PublicKey, StaticSecret};

    use crate::node::client_handling::client_registration::{
        Client, ClientMac, ClientMessage, InitMessage,
    };
    use crate::node::client_handling::client_registration::{ClientPublicKey, HmacSha256};
    use crate::node::http::{router_with_state, ApiState, ROUTE_PREFIX};

    #[tokio::test]
    async fn registration() {
        // 1. Provision random keys for gateway and client
        // 2. Generate DH shared secret
        // 3. Client submits its public key to the gateway to start the handshake process, gateway responds with nonce
        // 4. Client generates mac digest using DH shared secret, its own public key, socket address and port, and nonce
        // 5. Client sends its public key, socket address and port, nonce and mac digest to the gateway
        // 6. Gateway verifies mac digest and nonce, and stores client's public key and socket address and port

        let mut rng = rand::thread_rng();

        let gateway_key_pair = encryption::KeyPair::new(&mut rng);
        let client_key_pair = encryption::KeyPair::new(&mut rng);

        let gateway_static_public =
            PublicKey::try_from(gateway_key_pair.public_key().to_bytes()).unwrap();

        let client_static_private =
            StaticSecret::try_from(client_key_pair.private_key().to_bytes()).unwrap();
        let client_static_public =
            PublicKey::try_from(client_key_pair.public_key().to_bytes()).unwrap();

        let client_dh = client_static_private.diffie_hellman(&gateway_static_public);

        let registration_in_progress = Arc::new(RwLock::new(HashMap::new()));
        let client_registry = Arc::new(RwLock::new(HashMap::new()));

        let state = Arc::new(ApiState {
            client_registry: Arc::clone(&client_registry),
            sphinx_key_pair: Arc::new(gateway_key_pair),
            registration_in_progress: Arc::clone(&registration_in_progress),
        });

        // `Router` implements `tower::Service<Request<Body>>` so we can
        // call it like any tower service, no need to run an HTTP server.
        let mut app = router_with_state(state);

        let init_message =
            ClientMessage::Init(InitMessage::new(ClientPublicKey::new(client_static_public)));

        let init_request = Request::builder()
            .method("POST")
            .uri(format!("{}/client", ROUTE_PREFIX))
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
        assert!(!registration_in_progress.read().await.is_empty());

        let nonce: Option<u64> =
            serde_json::from_slice(&hyper::body::to_bytes(response.into_body()).await.unwrap())
                .unwrap();
        assert!(nonce.is_some());

        let mut mac = HmacSha256::new_from_slice(client_dh.as_bytes()).unwrap();
        mac.update(client_static_public.as_bytes());
        mac.update("127.0.0.1".as_bytes());
        mac.update("8080".as_bytes());
        mac.update(&nonce.unwrap().to_le_bytes());
        let mac = mac.finalize().into_bytes();

        let finalized_message = ClientMessage::Final(Client {
            pub_key: ClientPublicKey::new(client_static_public),
            socket: SocketAddr::from_str("127.0.0.1:8080").unwrap(),
            mac: ClientMac::new(mac.as_slice().to_vec()),
        });

        let final_request = Request::builder()
            .method("POST")
            .uri(format!("{}/client", ROUTE_PREFIX))
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
        assert!(!client_registry.read().await.is_empty());

        let clients_request = Request::builder()
            .method("GET")
            .uri(format!("{}/clients", ROUTE_PREFIX))
            .body(Body::empty())
            .unwrap();

        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(clients_request)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let clients: Vec<ClientPublicKey> =
            serde_json::from_slice(&hyper::body::to_bytes(response.into_body()).await.unwrap())
                .unwrap();

        assert!(!clients.is_empty());

        let ro_clients = client_registry.read().await.clone();
        assert_eq!(
            ro_clients
                .values()
                .map(|c| c.pub_key().clone())
                .collect::<Vec<ClientPublicKey>>(),
            clients
        )
    }
}
