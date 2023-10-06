use axum::extract::Path;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::{extract::State, Json};
use std::str::FromStr;
use tokio::sync::RwLock;

use crate::node::{Client, ClientPublicKey, ClientRegistry};

pub(crate) async fn register_client(
    State(registry): State<Arc<RwLock<ClientRegistry>>>,
    Json(payload): Json<Client>,
) -> StatusCode {
    let mut registry_rw = registry.write().await;
    registry_rw.insert(payload.socket, payload);
    StatusCode::OK
}

pub(crate) async fn get_all_clients(
    State(registry): State<Arc<RwLock<ClientRegistry>>>,
) -> (StatusCode, Json<ClientRegistry>) {
    let registry_ro = registry.read().await;
    (StatusCode::OK, Json(registry_ro.clone()))
}

pub(crate) async fn get_client(
    Path(pub_key): Path<String>,
    State(registry): State<Arc<RwLock<ClientRegistry>>>,
) -> (StatusCode, Json<Vec<Client>>) {
    let pub_key = match ClientPublicKey::from_str(&pub_key) {
        Ok(pub_key) => pub_key,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(vec![])),
    };
    let registry_ro = registry.read().await;
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
