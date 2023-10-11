// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::api::v1::gateway::client_interfaces::wireguard::{
    WireguardAppState, WireguardAppStateInner,
};
use crate::http::api::{FormattedResponse, OutputParams};
use crate::http::router::types::RequestError;
use crate::wireguard::error::WireguardError;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use nym_node_requests::api::v1::gateway::client_interfaces::wireguard::models::{
    Client, ClientMessage, ClientPublicKey, ClientRegistrationResponse, InitMessage, Nonce,
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
        (status = 501, body = ErrorResponse, description = "the endpoint hasn't been implemented yet"),
        (status = 400, body = ErrorResponse),
        (status = 200, content(
            ("application/json" = ClientRegistrationResponse),
            ("application/yaml" = ClientRegistrationResponse)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn register_client(
    State(state): State<WireguardAppState>,
    Query(output): Query<OutputParams>,
    Json(payload): Json<ClientMessage>,
) -> Result<RegisterClientResponse, RequestError> {
    let output = output.output.unwrap_or_default();

    let Some(state) = state.inner() else {
        return Err(RequestError::new_status(StatusCode::NOT_IMPLEMENTED));
    };

    match payload {
        ClientMessage::Initial(init) => {
            let nonce = process_init_message(init, state).await;
            let response = ClientRegistrationResponse::PendingRegistration { nonce };
            Ok(output.to_response(response))
        }
        ClientMessage::Final(finalize) => {
            let result = process_final_message(finalize, state).await?;
            if result.is_success() {
                let response = ClientRegistrationResponse::Registered { success: true };
                Ok(output.to_response(response))
            } else {
                Err(RequestError::new_status(result))
            }
        }
    }
}

pub type RegisterClientResponse = FormattedResponse<ClientRegistrationResponse>;

/// Get public keys of all registered wireguard clients.
#[utoipa::path(
    get,
    path = "/clients",
    context_path = "/api/v1/gateway/client-interfaces/wireguard",
    tag = "Wireguard (EXPERIMENTAL AND UNSTABLE)",
    responses(
        (status = 501, body = ErrorResponse, description = "the endpoint hasn't been implemented yet"),
        (status = 200, content(
            ("application/json" = Vec<String>),
            ("application/yaml" = Vec<String>)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn get_all_clients(
    Query(output): Query<OutputParams>,
    State(state): State<WireguardAppState>,
) -> Result<AllClientsResponse, RequestError> {
    let output = output.output.unwrap_or_default();

    let Some(state) = state.inner() else {
        return Err(RequestError::new_status(StatusCode::NOT_IMPLEMENTED));
    };

    let registry_ro = state.client_registry.read().await;
    let clients = registry_ro
        .values()
        .map(|c| c.pub_key())
        .collect::<Vec<ClientPublicKey>>();

    Ok(output.to_response(clients))
}

pub type AllClientsResponse = FormattedResponse<Vec<ClientPublicKey>>;

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
        (status = 501, body = ErrorResponse, description = "the endpoint hasn't been implemented yet"),
        (status = 404, body = ErrorResponse, description = "there are no clients with the provided public key"),
        (status = 400, body = ErrorResponse),
        (status = 200, content(
            ("application/json" = Vec<Client>),
            ("application/yaml" = Vec<Client>)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn get_client(
    Path(pub_key): Path<ClientPublicKey>,
    Query(output): Query<OutputParams>,
    State(state): State<WireguardAppState>,
) -> Result<ClientResponse, RequestError> {
    let output = output.output.unwrap_or_default();

    let Some(state) = state.inner() else {
        return Err(RequestError::new_status(StatusCode::NOT_IMPLEMENTED));
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
        return Err(RequestError::new_status(StatusCode::NOT_FOUND));
    }

    Ok(output.to_response(clients))
}

pub type ClientResponse = FormattedResponse<Vec<Client>>;
