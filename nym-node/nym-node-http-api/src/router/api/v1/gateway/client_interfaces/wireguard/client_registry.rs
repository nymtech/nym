// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::error::WireguardError;
use crate::api::v1::gateway::client_interfaces::wireguard::{
    WireguardAppState, WireguardAppStateInner,
};
use crate::api::{FormattedResponse, OutputParams};
use crate::router::types::RequestError;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use nym_node_requests::api::v1::gateway::client_interfaces::wireguard::models::{
    ClientMessage, ClientRegistrationResponse, GatewayClient, InitMessage, PeerPublicKey,
};
use nym_wireguard_types::registration::RegistrationData;
use rand::{prelude::IteratorRandom, thread_rng};

fn remove_from_registry(
    state: &WireguardAppStateInner,
    remote_public: &PeerPublicKey,
    gateway_client: &GatewayClient,
) -> Result<(), RequestError> {
    state
        .wireguard_gateway_data
        .remove_peer(gateway_client)
        .map_err(|err| RequestError::from_err(err, StatusCode::INTERNAL_SERVER_ERROR))?;
    state
        .wireguard_gateway_data
        .client_registry()
        .remove(&remote_public);
    Ok(())
}

async fn process_final_message(
    client: GatewayClient,
    state: &WireguardAppStateInner,
) -> Result<ClientRegistrationResponse, RequestError> {
    let registration_data = {
        if let Some(registration_data) = state.registration_in_progress.get(&client.pub_key()) {
            registration_data
        } else {
            return Err(RequestError::from_err(
                WireguardError::RegistrationNotInProgress,
                StatusCode::BAD_REQUEST,
            ));
        }
    };

    if client
        .verify(
            state.wireguard_gateway_data.keypair().private_key(),
            registration_data.nonce,
        )
        .is_ok()
    {
        state
            .wireguard_gateway_data
            .add_peer(&client)
            .map_err(|err| RequestError::from_err(err, StatusCode::INTERNAL_SERVER_ERROR))?;
        state.registration_in_progress.remove(&client.pub_key());
        state
            .wireguard_gateway_data
            .client_registry()
            .insert(client.pub_key(), client);

        Ok(ClientRegistrationResponse::Registered)
    } else {
        Err(RequestError::from_err(
            WireguardError::MacVerificationFailure,
            StatusCode::BAD_REQUEST,
        ))
    }
}

async fn process_init_message(
    init_message: InitMessage,
    state: &WireguardAppStateInner,
) -> Result<ClientRegistrationResponse, RequestError> {
    let remote_public = init_message.pub_key();
    let nonce: u64 = fastrand::u64(..);
    if let Some(registration_data) = state.registration_in_progress.get(&remote_public) {
        return Ok(ClientRegistrationResponse::PendingRegistration(
            registration_data.value().clone(),
        ));
    }
    let gateway_client_opt = if let Some(gateway_client) = state
        .wireguard_gateway_data
        .client_registry()
        .get(&remote_public)
    {
        let mut private_ip_ref = state
            .free_private_network_ips
            .get_mut(&gateway_client.private_ip)
            .ok_or(RequestError::new(
                "Internal data corruption",
                StatusCode::INTERNAL_SERVER_ERROR,
            ))?;
        *private_ip_ref = true;
        Some(gateway_client.clone())
    } else {
        None
    };
    if let Some(gateway_client) = gateway_client_opt {
        remove_from_registry(state, &remote_public, &gateway_client)?;
    }
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
        state.wireguard_gateway_data.keypair().private_key(),
        remote_public.inner(),
        *private_ip_ref.key(),
        nonce,
    );
    let registration_data = RegistrationData {
        nonce,
        gateway_data,
        wg_port: state.binding_port,
    };
    state
        .registration_in_progress
        .insert(remote_public, registration_data.clone());
    Ok(ClientRegistrationResponse::PendingRegistration(
        registration_data,
    ))
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

    let response = match payload {
        ClientMessage::Initial(init) => process_init_message(init, state).await?,
        ClientMessage::Final(finalize) => process_final_message(finalize, state).await?,
    };
    Ok(output.to_response(response))
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
        .wireguard_gateway_data
        .client_registry()
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
        .wireguard_gateway_data
        .client_registry()
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
