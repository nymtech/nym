// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use crate::support::caching::cache::UninitialisedCache;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::Router;
use nym_api_requests::models::ConfigScoreDataResponse;
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_mixnet_contract_common::NodeId;
use serde::Deserialize;
use utoipa::IntoParams;

pub(crate) mod network_monitor;
pub(crate) mod unstable;
pub(crate) mod without_monitor;

pub(crate) fn status_routes(network_monitor: bool) -> Router<AppState> {
    // in the minimal variant we would not have access to endpoints relying on existence
    // of the network monitor and the associated storage
    let without_network_monitor = without_monitor::mandatory_routes();

    if network_monitor {
        let with_network_monitor = network_monitor::network_monitor_routes();

        with_network_monitor.merge(without_network_monitor)
    } else {
        without_network_monitor
    }
    .route("/config-score-details", get(config_score_details))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
struct MixIdParam {
    mix_id: NodeId,
}

#[utoipa::path(
    tag = "Status",
    get,
    path = "/config-score-details",
    context_path = "/v1/status",
    responses(
        (status = 200, content(
            (ConfigScoreDataResponse = "application/json"),
            (ConfigScoreDataResponse = "application/yaml"),
            (ConfigScoreDataResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn config_score_details(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<ConfigScoreDataResponse>> {
    let output = output.output.unwrap_or_default();

    let data = state
        .nym_contract_cache()
        .maybe_config_score_data_owned()
        .await
        .ok_or(UninitialisedCache)?;

    Ok(output.to_response(data.into_inner().into()))
}
