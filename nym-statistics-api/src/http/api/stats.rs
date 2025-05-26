use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, State},
    Json, Router,
};
use nym_statistics_common::report::vpn_client::VpnClientStatsReport;

use crate::http::{
    error::{HttpError, HttpResult},
    state::AppState,
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/report", axum::routing::post(submit_stats_report))
}

#[utoipa::path(
    post,
    request_body = u32,
    path = "/v1/stats/report",
    responses(
        (status = 200)
    )
)]
#[tracing::instrument(level = "debug", skip_all)]
async fn submit_stats_report(
    State(mut state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(report): Json<VpnClientStatsReport>,
) -> HttpResult<Json<()>> {
    // SW TODO use addr to whitelist gateways

    state
        .storage()
        .store_vpn_client_report(report, time::OffsetDateTime::now_utc(), addr)
        .await
        .map_err(HttpError::internal_with_logging)?;

    Ok(Json(()))
}
