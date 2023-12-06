// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::v1::gateway::client_interfaces::wireguard::{
    WireguardAppState, WireguardAppStateInner,
};
use crate::http::api::{FormattedResponse, OutputParams};
use crate::http::router::types::RequestError;
use crate::wireguard::error::WireguardError;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use nym_crypto::asymmetric::encryption::PublicKey;
use nym_node_requests::api::v1::gateway::client_interfaces::wireguard::models::{
    ClientMessage, ClientRegistrationResponse, GatewayClient, InitMessage, Nonce, PeerPublicKey,
};
use rand::{prelude::IteratorRandom, thread_rng};

async fn process_final_message(
    client: GatewayClient,
    state: &WireguardAppStateInner,
) -> Result<StatusCode, RequestError> {
    let preshared_nonce = {
        if let Some(nonce) = state.registration_in_progress.get(&client.pub_key()) {
            *nonce
        } else {
            return Err(RequestError::from_err(
                WireguardError::RegistrationNotInProgress,
                StatusCode::BAD_REQUEST,
            ));
        }
    };

    if client.verify(&state.private_key, preshared_nonce).is_ok() {
        state.registration_in_progress.remove(&client.pub_key());
        state.client_registry.insert(client.pub_key(), client);

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
    state
        .registration_in_progress
        .insert(init_message.pub_key(), nonce);
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
            let remote_public = PublicKey::from_bytes(init.pub_key().as_bytes())
                .map_err(|_| RequestError::new_status(StatusCode::BAD_REQUEST))?;
            let nonce = process_init_message(init, state).await;
            let mut private_ip_ref = state
                .free_private_network_ips
                .iter_mut()
                .filter(|r| **r)
                .choose(&mut thread_rng())
                .ok_or(RequestError::new(
                    "No more space in the network",
                    StatusCode::SERVICE_UNAVAILABLE,
                ))?;
            // mark it as used, even though it's not final
            *private_ip_ref = false;
            let gateway_data = GatewayClient::new(
                &state.private_key,
                remote_public,
                *private_ip_ref.key(),
                nonce,
            );
            let response = ClientRegistrationResponse::PendingRegistration {
                nonce,
                gateway_data,
                wg_port: state.binding_port,
            };
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

    let clients = state
        .client_registry
        .iter()
        .map(|c| c.pub_key())
        .collect::<Vec<PeerPublicKey>>();

    Ok(output.to_response(clients))
}

pub type AllClientsResponse = FormattedResponse<Vec<PeerPublicKey>>;

/// Get client details of the registered wireguard client by its public key.
#[utoipa::path(
    get,
    path = "/client/{pub_key}",
    context_path = "/api/v1/gateway/client-interfaces/wireguard",
    tag = "Wireguard (EXPERIMENTAL AND UNSTABLE)",
    params(
        ("pub_key", Path, description = "The public key of the client"),
        OutputParams
    ),
    responses(
        (status = 501, body = ErrorResponse, description = "the endpoint hasn't been implemented yet"),
        (status = 404, body = ErrorResponse, description = "there are no clients with the provided public key"),
        (status = 400, body = ErrorResponse),
        (status = 200, content(
            ("application/json" = Vec<GatewayClient>),
            ("application/yaml" = Vec<GatewayClient>)
        ))
    ),
)]
pub(crate) async fn get_client(
    Path(pub_key): Path<PeerPublicKey>,
    Query(output): Query<OutputParams>,
    State(state): State<WireguardAppState>,
) -> Result<ClientResponse, RequestError> {
    let output = output.output.unwrap_or_default();

    let Some(state) = state.inner() else {
        return Err(RequestError::new_status(StatusCode::NOT_IMPLEMENTED));
    };

    let clients = state
        .client_registry
        .iter()
        .filter_map(|c| {
            if c.pub_key() == pub_key {
                Some(c.clone())
            } else {
                None
            }
        })
        .collect::<Vec<GatewayClient>>();

    if clients.is_empty() {
        return Err(RequestError::new_status(StatusCode::NOT_FOUND));
    }

    Ok(output.to_response(clients))
}

pub type ClientResponse = FormattedResponse<Vec<GatewayClient>>;
