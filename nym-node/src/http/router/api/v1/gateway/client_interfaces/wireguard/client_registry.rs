// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::api::v1::gateway::client_interfaces::wireguard::{
    WireguardAppState, WireguardAppStateInner,
};
use crate::http::router::types::RequestError;
use crate::wireguard::error::WireguardError;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use nym_node_requests::api::v1::gateway::client_interfaces::wireguard::models::{
    Client, ClientMessage, ClientPublicKey, InitMessage, Nonce,
};

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

async fn process_init_message(init_message: InitMessage, state: &WireguardAppStateInner) -> Nonce {
    let nonce: u64 = fastrand::u64(..);
    let mut registry_rw = state.registration_in_progress.write().await;
    registry_rw.insert(init_message.pub_key(), nonce);
    nonce
}

/// Perform wireguard client registration.
#[utoipa::path(
    post,
    path = "/client",
    context_path = "/api/v1/gateway/client-interfaces/wireguard",
    tag = "Wireguard (EXPERIMENTAL AND UNSTABLE)",
    request_body(
        content = ClientMessage,
        description = "Data used for proceeding with client wireguard registration",
        content_type = "application/json"
    ),
    responses(
        (status = 501, description = "the endpoint hasn't been implemented yet"),
        (status = 400, body = ErrorResponse),
        (status = 200, content(
            ("application/json" = Option<u64>),
            // ("application/yaml" = ClientInterfaces)
        ))
    ),
    // params(OutputParams)
)]
pub(crate) async fn register_client(
    State(state): State<WireguardAppState>,
    Json(payload): Json<ClientMessage>,
) -> Result<(StatusCode, Json<Option<Nonce>>), RequestError> {
    let Some(state) = state.inner() else {
        return Ok((StatusCode::NOT_IMPLEMENTED, Json(None)));
    };

    match payload {
        ClientMessage::Initial(init) => Ok((
            StatusCode::OK,
            Json(Some(process_init_message(init, state).await)),
        )),
        ClientMessage::Final(finalize) => process_final_message(finalize, state)
            .await
            .map(|code| (code, Json(None))),
    }
}

// pub type RegisterClientResponse = FormattedResponse<()>;

/// Get public keys of all registered wireguard clients.
#[utoipa::path(
    get,
    path = "/clients",
    context_path = "/api/v1/gateway/client-interfaces/wireguard",
    tag = "Wireguard (EXPERIMENTAL AND UNSTABLE)",
    responses(
        (status = 501, description = "the endpoint hasn't been implemented yet"),
        (status = 200, content(
            ("application/json" = Vec<String>),
            // ("application/yaml" = ClientInterfaces)
        ))
    ),
    // params(OutputParams)
)]
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

/// Get client details of the registered wireguard client by its public key.
#[utoipa::path(
    get,
    path = "/client/{pub_key}",
    context_path = "/api/v1/gateway/client-interfaces/wireguard",
    tag = "Wireguard (EXPERIMENTAL AND UNSTABLE)",
    params(
        ("pub_key", description = "The public key of the client"),
    ),
    responses(
        (status = 501, description = "the endpoint hasn't been implemented yet"),
        (status = 404, description = "there are no clients with the provided public key"),
        (status = 400, body = ErrorResponse),
        (status = 200, content(
            ("application/json" = Vec<Client>),
            // ("application/yaml" = ClientInterfaces)
        ))
    ),
    // params(OutputParams)
)]
pub(crate) async fn get_client(
    Path(pub_key): Path<ClientPublicKey>,
    State(state): State<WireguardAppState>,
) -> Result<(StatusCode, Json<Vec<Client>>), RequestError> {
    let Some(state) = state.inner() else {
        return Ok((StatusCode::NOT_IMPLEMENTED, Json(Vec::new())));
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
