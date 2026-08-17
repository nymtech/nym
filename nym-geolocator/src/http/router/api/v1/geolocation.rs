// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::error::{ErrorResponse, RequestError};
use crate::http::state::AppState;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use nym_geolocation_contract_common::NymNodeLocation;
use nym_geolocation_contract_common::payload::Location;
use nym_geolocator_requests::models::{
    MeasurementResponse, RecheckNodeRequest, RelayResponse, SignedCheckRequest,
};
use nym_geolocator_requests::routes::api::v1::geolocation::{
    RECHECK_NODE, RELAY_SELF_DECLARATION, REQUEST_CHECK,
};
use nym_http_api_common::middleware::bearer_auth::AuthLayer;
use nym_validator_client::nyxd::nym_performance_contract_common::NodeId;

pub(crate) fn routes(recheck_node_auth: AuthLayer) -> Router<AppState> {
    Router::new()
        .route(REQUEST_CHECK, post(request_geolocation_check))
        .route(RECHECK_NODE, post(recheck_node).layer(recheck_node_auth))
        .route(RELAY_SELF_DECLARATION, post(relay_self_declaration))
}

/// Measure the requesting node's location now, on the authority of its own identity key.
///
/// A node can only ever request a measurement of itself: the signature is verified against the
/// identity key of the node named in the body, so one naming a different node is checked against
/// that node's key and simply fails to verify. There is no separate "is the signer the subject"
/// test to get wrong.
#[utoipa::path(
    post,
    path = "/request-check",
    context_path = "/api/v1/geolocation",
    tag = "Geolocation",
    request_body = SignedCheckRequest,
    responses(
        (status = 200, body = MeasurementResponse, description = "the measurement was submitted to the contract"),
        (status = 401, body = ErrorResponse, description = "the signature, its timestamp or its freshness was rejected"),
        (status = 404, body = ErrorResponse, description = "the node is not bonded"),
        (status = 429, body = ErrorResponse, description = "the node has spent its re-test allowance and is in cooldown"),
        (status = 502, body = ErrorResponse, description = "the node could not be reached, located or submitted for"),
        (status = 503, body = ErrorResponse, description = "the lookup provider was busy with other work - nothing was attempted, retry"),
    ),
)]
async fn request_geolocation_check(
    State(state): State<AppState>,
    Json(body): Json<SignedCheckRequest>,
) -> Result<Json<MeasurementResponse>, RequestError> {
    let node_id = body.node_id;

    // the bond is what carries the identity key, so an unbonded node cannot authenticate here at
    // all. it would also be pointless to measure one: the contract deleted its entries when it
    // unbonded and cannot delete them a second time
    let Some(bond) = state.scraper.bonded_node(node_id).await else {
        return Err(RequestError::not_found(format!(
            "node {node_id} is not bonded"
        )));
    };

    if body.verify(&bond.identity).is_err() {
        return Err(RequestError::unauthorised(format!(
            "the request was not signed by the identity key of node {node_id}"
        )));
    }

    // strictly after the signature check: recording an unverified request would let anyone
    // advance a node's timestamp with a forgery and lock the real node out at will
    state
        .replay_guard
        .accept_once(node_id, body.signed_at)
        .await?;

    // after the replay check, so a replayed request is turned away without touching the allowance,
    // and charged before the measurement rather than after it, so that requests still in flight
    // cannot all pass the check together
    state.burst_limiter.claim_allowance(node_id).await?;

    let (location, changed) = match measure_claimed(&state, node_id).await {
        Ok(measured) => measured,
        Err(err) => {
            state.burst_limiter.release_claim(node_id).await;
            return Err(err);
        }
    };

    if changed {
        state.burst_limiter.restore_allowance(node_id).await;
    }

    Ok(Json(MeasurementResponse { node_id, location }))
}

/// Measure a node whose allowance has already been claimed, reporting whether the result differs
/// from what this agent has stored for it.
///
/// Separated only so that every way of failing releases the claim, rather than each having to
/// remember to.
async fn measure_claimed(
    state: &AppState,
    node_id: NodeId,
) -> Result<(Location, bool), RequestError> {
    // read before measuring, since submitting overwrites the very value being compared against.
    // a failure here also stops us spending a metered lookup we could not then account for
    let stored = state.stored_measurement(node_id).await?;

    let location = state.measure_and_submit(node_id).await?;

    // a node with nothing stored yet counts as changed: it is asking for a first measurement, not
    // re-asking for one it already has
    let changed = stored.as_ref() != Some(&location);

    Ok((location, changed))
}

/// Measure a node's location now, bypassing the sweep schedule and any rate limiting.
///
/// The bearer token is held by the operator of this service, so no burst limit applies: the
/// counter exists to stop nodes from spending our metered lookup allowance on themselves, and an
/// operator locking themselves out of their own tooling serves nobody.
#[utoipa::path(
    post,
    path = "/recheck-node",
    context_path = "/api/v1/geolocation",
    tag = "Geolocation",
    request_body = RecheckNodeRequest,
    responses(
        (status = 200, body = MeasurementResponse, description = "the measurement was submitted to the contract"),
        (status = 401, description = "the bearer token was missing or invalid"),
        (status = 404, body = ErrorResponse, description = "the node is not bonded"),
        (status = 502, body = ErrorResponse, description = "the node could not be reached, located or submitted for"),
        (status = 503, body = ErrorResponse, description = "the lookup provider was busy with other work - nothing was attempted, retry"),
    ),
    security(
        ("admin_token" = [])
    )
)]
async fn recheck_node(
    State(state): State<AppState>,
    Json(body): Json<RecheckNodeRequest>,
) -> Result<Json<MeasurementResponse>, RequestError> {
    let node_id = body.node_id;

    // the contract deletes a node's entries itself when it unbonds, and the measurement path
    // performs no bonding check of its own, so submitting for an unbonded node would resurrect an
    // entry that nothing can ever delete again
    if state.scraper.bonded_node(node_id).await.is_none() {
        return Err(RequestError::not_found(format!(
            "node {node_id} is not bonded"
        )));
    }

    let location = state.measure_and_submit(node_id).await?;

    Ok(Json(MeasurementResponse { node_id, location }))
}

/// Relay a node's own signed location declaration to the contract, unchanged.
///
/// The node pushes its artifact here rather than this service polling for it: a declaration only
/// changes when its operator changes it, so polling would spend a request per node per cycle to
/// learn nothing almost every time, and would still relay a change no sooner than the next poll.
///
/// Nothing on this path decodes the payload. The node signed those exact bytes, and JSON key
/// order, whitespace and float formatting all vary between implementations, so a payload that was
/// parsed and re-emitted could differ from the signed original and fail verification on chain.
/// `LocationPayload` carries its `content` as opaque bytes precisely so that cannot happen here.
#[utoipa::path(
    post,
    path = "/relay-self-declaration",
    context_path = "/api/v1/geolocation",
    tag = "Geolocation",
    request_body = NymNodeLocation,
    responses(
        (status = 200, body = RelayResponse, description = "the declaration was relayed to the contract"),
        (status = 400, body = ErrorResponse, description = "the payload is oversized or declared too far ahead"),
        (status = 401, body = ErrorResponse, description = "the declaration was not signed by the node it names"),
        (status = 403, body = ErrorResponse, description = "this agent is not authorised to relay declarations"),
        (status = 404, body = ErrorResponse, description = "the node is not bonded"),
        (status = 409, body = ErrorResponse, description = "a declaration at least as recent is already stored"),
        (status = 502, body = ErrorResponse, description = "the declaration could not be relayed"),
    ),
)]
async fn relay_self_declaration(
    State(state): State<AppState>,
    Json(declaration): Json<NymNodeLocation>,
) -> Result<Json<RelayResponse>, RequestError> {
    let node_id = declaration.node_id;
    let declared_at = declaration.declared_at;

    // the contract resolves the identity key from the mixnet contract itself, so this both
    // supplies the key to check against and mirrors the check the contract will make
    let Some(bond) = state.scraper.bonded_node(node_id).await else {
        return Err(RequestError::not_found(format!(
            "node {node_id} is not bonded"
        )));
    };

    // checked before the artifact is examined at all: if this agent cannot relay, nothing about
    // the declaration can change that, and the node should be told to go to a different agent
    state.ensure_can_relay().await?;

    state
        .prevalidate_declaration(&declaration, &bond.identity)
        .await?;

    state.relay_declaration(declaration).await?;

    Ok(Json(RelayResponse {
        node_id,
        declared_at,
    }))
}
