// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::AppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::v1::network::models::UpgradeModeStatus;

/// Returns current upgrade mode information as perceived by this node.
#[utoipa::path(
    get,
    path = "/upgrade-mode-status",
    context_path = "/api/v1/network",
    tag = "Network",
    responses(
        (status = 200, content(
            (UpgradeModeStatus = "application/json"),
            (UpgradeModeStatus = "application/yaml"),
            (UpgradeModeStatus = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn upgrade_mode_status(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> UpgradeModeStatusResponse {
    let output = output.get_output();
    let um_state = &state.upgrade_mode_state.node_state;
    output.to_response(UpgradeModeStatus {
        enabled: um_state.upgrade_mode_enabled(),
        last_queried: um_state.last_queried(),
        attestation_provider: state.upgrade_mode_state.attestation_url.clone(),
        attester_pubkey: um_state.attester_pubkey(),
        published_attestation: um_state.attestation().await,
    })
}

pub type UpgradeModeStatusResponse = FormattedResponse<UpgradeModeStatus>;
