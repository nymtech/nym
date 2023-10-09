use axum::extract::Path;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::{extract::State, Json};
use std::str::FromStr;

// use axum_macros::debug_handler;

use crate::node::client_handling::client_registration::{
    Client, ClientMessage, ClientPublicKey, InitMessage,
};
use crate::node::http::ApiState;

async fn process_final_message(client: Client, state: Arc<ApiState>) -> StatusCode {
    let preshared_nonce = {
        let in_progress_ro = state.registration_in_progress.read().await;
        if let Some(nonce) = in_progress_ro.get(client.pub_key()) {
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
            let mut in_progress_rw = state.registration_in_progress.write().await;
            in_progress_rw.remove(client.pub_key());
        }
        {
            let mut registry_rw = state.client_registry.write().await;
            registry_rw.insert(client.socket(), client);
        }
        return StatusCode::OK;
    }

    StatusCode::BAD_REQUEST
}

async fn process_init_message(init_message: InitMessage, state: Arc<ApiState>) -> u64 {
    let nonce: u64 = fastrand::u64(..);
    let mut registry_rw = state.registration_in_progress.write().await;
    registry_rw.insert(init_message.pub_key().clone(), nonce);
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
) -> (StatusCode, Json<Vec<ClientPublicKey>>) {
    let registry_ro = state.client_registry.read().await;
    (
        StatusCode::OK,
        Json(
            registry_ro
                .values()
                .map(|c| c.pub_key().clone())
                .collect::<Vec<ClientPublicKey>>(),
        ),
    )
}

pub(crate) async fn get_client(
    Path(pub_key): Path<String>,
    State(state): State<Arc<ApiState>>,
) -> (StatusCode, Json<Vec<Client>>) {
    let pub_key = match ClientPublicKey::from_str(&pub_key) {
        Ok(pub_key) => pub_key,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(vec![])),
    };
    let registry_ro = state.client_registry.read().await;
    let clients = registry_ro
        .iter()
        .filter_map(|(_, c)| {
            if c.pub_key() == &pub_key {
                Some(c.clone())
            } else {
                None
            }
        })
        .collect::<Vec<Client>>();
    if clients.is_empty() {
        return (StatusCode::NOT_FOUND, Json(clients));
    }
    (StatusCode::OK, Json(clients))
}
