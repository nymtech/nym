use axum::extract::Path;
use nym_types::gateway_client_registration::{ClientMessage, GatewayClient, InitMessage};
use nym_types::wireguard::PeerPublicKey;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::{extract::State, Json};
use std::str::FromStr;

// use axum_macros::debug_handler;

use crate::node::http::ApiState;

async fn process_final_message(client: GatewayClient, state: Arc<ApiState>) -> StatusCode {
    let preshared_nonce = {
        if let Some(nonce) = state.registration_in_progress.get(client.pub_key()) {
            *nonce
        } else {
            return StatusCode::BAD_REQUEST;
        }
    };

    if client
        .verify(state.sphinx_key_pair.private_key(), preshared_nonce)
        .is_ok()
    {
        {
            state.registration_in_progress.remove(client.pub_key());
        }
        {
            state
                .client_registry
                .insert(client.pub_key().clone(), client);
        }
        return StatusCode::OK;
    }

    StatusCode::BAD_REQUEST
}

async fn process_init_message(init_message: InitMessage, state: Arc<ApiState>) -> u64 {
    let nonce: u64 = fastrand::u64(..);
    state
        .registration_in_progress
        .insert(init_message.pub_key().clone(), nonce);
    nonce
}

// #[debug_handler]
pub(crate) async fn register_client(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<ClientMessage>,
) -> (StatusCode, Json<Option<u64>>) {
    match payload {
        ClientMessage::Init(i) => (
            StatusCode::OK,
            Json(Some(process_init_message(i, Arc::clone(&state)).await)),
        ),
        ClientMessage::Final(client) => (
            process_final_message(client, Arc::clone(&state)).await,
            Json(None),
        ),
    }
}

pub(crate) async fn get_all_clients(
    State(state): State<Arc<ApiState>>,
) -> (StatusCode, Json<Vec<PeerPublicKey>>) {
    (
        StatusCode::OK,
        Json(
            state
                .client_registry
                .iter()
                .map(|c| c.pub_key().clone())
                .collect::<Vec<PeerPublicKey>>(),
        ),
    )
}

pub(crate) async fn get_client(
    Path(pub_key): Path<String>,
    State(state): State<Arc<ApiState>>,
) -> (StatusCode, Json<Vec<GatewayClient>>) {
    let pub_key = match PeerPublicKey::from_str(&pub_key) {
        Ok(pub_key) => pub_key,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(vec![])),
    };
    let clients = state
        .client_registry
        .iter()
        .filter_map(|r| {
            let client = r.value();
            if client.pub_key() == &pub_key {
                Some(client.clone())
            } else {
                None
            }
        })
        .collect::<Vec<GatewayClient>>();
    if clients.is_empty() {
        return (StatusCode::NOT_FOUND, Json(clients));
    }
    (StatusCode::OK, Json(clients))
}
