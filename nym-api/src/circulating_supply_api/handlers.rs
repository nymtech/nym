// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::Router;
use nym_api_requests::models::CirculatingSupplyResponse;
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_validator_client::nyxd::Coin;

pub(crate) fn circulating_supply_routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(get_full_circulating_supply))
        .route(
            "/circulating-supply-value",
            axum::routing::get(get_circulating_supply),
        )
        .route("/total-supply-value", axum::routing::get(get_total_supply))
}

#[utoipa::path(
    tag = "circulating-supply",
    get,
    path = "/v1/circulating-supply",
    responses(
        (status = 200, content(
            (CirculatingSupplyResponse = "application/json"),
            (CirculatingSupplyResponse = "application/yaml"),
            (CirculatingSupplyResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn get_full_circulating_supply(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<CirculatingSupplyResponse>> {
    let output = output.output.unwrap_or_default();

    let supply = state.latest_snapshot().await?.mixnet().circulating_supply();
    Ok(output.to_response(supply))
}

#[utoipa::path(
    tag = "circulating-supply",
    get,
    path = "/v1/circulating-supply/total-supply-value",
    responses(
        (status = 200, content(
            (f64 = "application/json"),
            (f64 = "application/yaml"),
            (f64 = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn get_total_supply(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<f64>> {
    let output = output.output.unwrap_or_default();
    let full_circulating_supply = state.latest_snapshot().await?.mixnet().circulating_supply();

    let total_supply = unym_coin_to_float_unym(full_circulating_supply.total_supply.into());

    Ok(output.to_response(total_supply))
}

#[utoipa::path(
    tag = "circulating-supply",
    get,
    path = "/v1/circulating-supply/circulating-supply-value",
    responses(
        (status = 200, content(
            (f64 = "application/json"),
            (f64 = "application/yaml"),
            (f64 = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn get_circulating_supply(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<f64>> {
    let output = output.output.unwrap_or_default();

    let full_circulating_supply = state.latest_snapshot().await?.mixnet().circulating_supply();

    let circulating_supply =
        unym_coin_to_float_unym(full_circulating_supply.circulating_supply.into());

    Ok(output.to_response(circulating_supply))
}

// TODO: this is not the best place to put it, it should be more centralised,
// but for a quick fix, that's good enough for now...
// (for proper solution we should be managing `NymNetworkDetails` via rocket and grabbing display exponent
// value from the mix denom here.
const UNYM_RATIO: f64 = 1000000.;

fn unym_coin_to_float_unym(coin: Coin) -> f64 {
    // our total supply can't exceed 1B so an overflow here is impossible
    // (if it happened, then we SHOULD crash)
    coin.amount as f64 / UNYM_RATIO
}
