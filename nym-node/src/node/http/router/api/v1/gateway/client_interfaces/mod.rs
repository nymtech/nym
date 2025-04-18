// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::v1::gateway::models::{ClientInterfaces, WebSockets, Wireguard};
use nym_node_requests::routes::api::v1::gateway::client_interfaces;

pub(crate) fn routes<S: Send + Sync + 'static + Clone>(
    interfaces: Option<ClientInterfaces>,
) -> Router<S> {
    Router::new()
        .route(
            "/",
            get({
                let interfaces = interfaces.clone();
                move |query| client_interfaces(interfaces, query)
            }),
        )
        .route(
            client_interfaces::WEBSOCKETS,
            get({
                let websockets = interfaces.as_ref().and_then(|i| i.mixnet_websockets);
                move |query| mixnet_websockets(websockets, query)
            }),
        )
        .route(
            client_interfaces::WIREGUARD,
            get({
                let wireguard = interfaces.as_ref().and_then(|i| i.wireguard.clone());
                move |query| wireguard_details(wireguard, query)
            }),
        )
}

/// Returns client interfaces supported by this gateway.
#[utoipa::path(
    get,
    path = "/client-interfaces",
    context_path = "/api/v1/gateway",
    tag = "Gateway",
    responses(
    (status = 501, description = "the endpoint hasn't been implemented yet"),
        (status = 200, content(
            (ClientInterfaces = "application/json"),
            (ClientInterfaces = "application/yaml")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn client_interfaces(
    interfaces: Option<ClientInterfaces>,
    Query(output): Query<OutputParams>,
) -> Result<ClientInterfacesResponse, StatusCode> {
    let interfaces = interfaces.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(interfaces))
}

pub type ClientInterfacesResponse = FormattedResponse<ClientInterfaces>;

/// Returns client interfaces supported by this gateway.
#[utoipa::path(
    get,
    path = "/mixnet-websockets",
    context_path = "/api/v1/gateway/client-interfaces",
    tag = "Gateway",
    responses(
        (status = 501, description = "the endpoint hasn't been implemented yet"),
        (status = 200, content(
            (WebSockets = "application/json"),
            (WebSockets = "application/yaml")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn mixnet_websockets(
    websockets: Option<WebSockets>,
    Query(output): Query<OutputParams>,
) -> Result<MixnetWebSocketsResponse, StatusCode> {
    let websockets = websockets.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(websockets))
}

pub type MixnetWebSocketsResponse = FormattedResponse<WebSockets>;

/// Returns wireguard information on this gateway.
#[utoipa::path(
    get,
    path = "/wireguard",
    context_path = "/api/v1/gateway/client-interfaces",
    tag = "Gateway",
    responses(
        (status = 501, description = "the endpoint hasn't been implemented yet"),
        (status = 200, content(
            (Wireguard = "application/json"),
            (Wireguard = "application/yaml")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn wireguard_details(
    wireguard: Option<Wireguard>,
    Query(output): Query<OutputParams>,
) -> Result<WireguardResponse, StatusCode> {
    let wireguard = wireguard.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(wireguard))
}

pub type WireguardResponse = FormattedResponse<Wireguard>;
