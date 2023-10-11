// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::api::v1::gateway::client_interfaces::wireguard::{
    WireguardAppState, WireguardAppStateInner,
};
use crate::wireguard::types::{
    Client, ClientMessage, ClientMessageRequest, ClientPublicKey, InitMessage,
};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use std::str::FromStr;
use tracing::warn;

async fn process_final_message(client: Client, state: &WireguardAppStateInner) -> StatusCode {
    let preshared_nonce = {
        let in_progress_ro = state.registration_in_progress.read().await;
        if let Some(nonce) = in_progress_ro.get(&client.pub_key()) {
            *nonce
        } else {
            return StatusCode::BAD_REQUEST;
        }
    };

    if client
        .verify(state.dh_keypair.private_key(), preshared_nonce)
        .is_ok()
    {
        {
            let mut in_progress_rw = state.registration_in_progress.write().await;
            in_progress_rw.remove(&client.pub_key());
        }
        {
            let mut registry_rw = state.client_registry.write().await;
            registry_rw.insert(client.socket(), client);
        }
        return StatusCode::OK;
    }

    StatusCode::BAD_REQUEST
}

async fn process_init_message(init_message: InitMessage, state: &WireguardAppStateInner) -> u64 {
    let nonce: u64 = fastrand::u64(..);
    let mut registry_rw = state.registration_in_progress.write().await;
    registry_rw.insert(*init_message.pub_key(), nonce);
    nonce
}

pub(crate) async fn register_client(
    State(state): State<WireguardAppState>,
    Json(payload): Json<ClientMessageRequest>,
) -> (StatusCode, Json<Option<u64>>) {
    let Some(state) = state.inner() else {
        return (StatusCode::NOT_IMPLEMENTED, Json(None));
    };

    match ClientMessage::try_from(payload) {
        Ok(payload) => match payload {
            ClientMessage::Init(i) => (
                StatusCode::OK,
                Json(Some(process_init_message(i, state).await)),
            ),
            ClientMessage::Final(client) => {
                (process_final_message(client, state).await, Json(None))
            }
        },
        Err(err) => {
            warn!("failed to deserialize received request: {err}");
            (StatusCode::BAD_REQUEST, Json(None))
        }
    }
}

// pub type RegisterClientResponse = FormattedResponse<()>;
pub(crate) async fn get_all_clients(
    State(state): State<WireguardAppState>,
) -> (StatusCode, Json<Vec<ClientPublicKey>>) {
    let Some(state) = state.inner() else {
        return (StatusCode::NOT_IMPLEMENTED, Json(Vec::new()));
    };
    let registry_ro = state.client_registry.read().await;
    (
        StatusCode::OK,
        Json(
            registry_ro
                .values()
                .map(|c| c.pub_key())
                .collect::<Vec<ClientPublicKey>>(),
        ),
    )
}

// pub type AllClientsResponse = FormattedResponse<()>;

pub(crate) async fn get_client(
    Path(pub_key): Path<String>,
    State(state): State<WireguardAppState>,
) -> (StatusCode, Json<Vec<Client>>) {
    let Some(state) = state.inner() else {
        return (StatusCode::NOT_IMPLEMENTED, Json(Vec::new()));
    };
    let pub_key = match ClientPublicKey::from_str(&pub_key) {
        Ok(pub_key) => pub_key,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(vec![])),
    };

    let registry_ro = state.client_registry.read().await;
    let clients = registry_ro
        .iter()
        .filter_map(|(_, c)| {
            if c.pub_key() == pub_key {
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

// pub type ClientResponse = FormattedResponse<()>;
