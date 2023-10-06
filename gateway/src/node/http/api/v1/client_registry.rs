use axum::extract::Path;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::{extract::State, Json};
use std::str::FromStr;

use crate::node::http::ApiState;
use crate::node::{Client, ClientPublicKey};

pub(crate) async fn register_client(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<Client>,
) -> StatusCode {
    let mut registry_rw = state.client_registry.write().await;
    if payload.verify(state.sphinx_key_pair.private_key()).is_ok() {
        registry_rw.insert(payload.socket, payload);
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
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
                .map(|c| c.pub_key.clone())
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
            if c.pub_key == pub_key {
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
