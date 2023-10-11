// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::api::v1::gateway::client_interfaces::wireguard::{
    WireguardAppState, WireguardAppStateInner,
};
use crate::http::router::types::RequestError;
use crate::wireguard::error::WireguardError;
use crate::wireguard::types::{
    Client, ClientMessage, ClientMessageRequest, ClientPublicKey, InitMessage,
};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use std::str::FromStr;

async fn process_final_message(
    client: Client,
    state: &WireguardAppStateInner,
) -> Result<StatusCode, RequestError> {
    let preshared_nonce = {
        let in_progress_ro = state.registration_in_progress.read().await;
        if let Some(nonce) = in_progress_ro.get(&client.pub_key()) {
            *nonce
        } else {
            return Err(RequestError::from_err(
                WireguardError::RegistrationNotInProgress,
                StatusCode::BAD_REQUEST,
            ));
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
        Ok(StatusCode::OK)
    } else {
        Err(RequestError::from_err(
            WireguardError::MacVerificationFailure,
            StatusCode::BAD_REQUEST,
        ))
    }
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
) -> Result<(StatusCode, Json<Option<u64>>), RequestError> {
    let Some(state) = state.inner() else {
        return Ok((StatusCode::NOT_IMPLEMENTED, Json(None)));
    };

    match ClientMessage::try_from(payload) {
        Ok(payload) => match payload {
            ClientMessage::Init(i) => Ok((
                StatusCode::OK,
                Json(Some(process_init_message(i, state).await)),
            )),
            ClientMessage::Final(client) => process_final_message(client, state)
                .await
                .map(|code| (code, Json(None))),
        },
        Err(err) => Err(RequestError::from_err(err, StatusCode::BAD_REQUEST)),
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
) -> Result<(StatusCode, Json<Vec<Client>>), RequestError> {
    let Some(state) = state.inner() else {
        return Ok((StatusCode::NOT_IMPLEMENTED, Json(Vec::new())));
    };
    let pub_key = match ClientPublicKey::from_str(&pub_key) {
        Ok(pub_key) => pub_key,
        Err(err) => return Err(RequestError::from_err(err, StatusCode::BAD_REQUEST)),
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
        return Ok((StatusCode::NOT_FOUND, Json(clients)));
    }
    Ok((StatusCode::OK, Json(clients)))
}

// pub type ClientResponse = FormattedResponse<()>;
