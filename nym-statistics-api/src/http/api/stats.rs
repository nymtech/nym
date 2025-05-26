use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, State},
    Json, Router,
};
use axum_extra::{headers::UserAgent, TypedHeader};
use nym_statistics_common::report::vpn_client::VpnClientStatsReport;
use tracing::debug;

use crate::{
    http::{
        error::{HttpError, HttpResult},
        state::AppState,
    },
    storage::models::{ConnectionInfoDto, DailyActiveDeviceDto},
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/report", axum::routing::post(submit_stats_report))
}

#[utoipa::path(
    post,
    request_body = VpnClientStatsReport,
    path = "/v1/stats/report",
    responses(
        (status = 200)
    )
)]
#[tracing::instrument(level = "debug", skip_all)]
async fn submit_stats_report(
    State(mut state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    Json(report): Json<VpnClientStatsReport>,
) -> HttpResult<Json<()>> {
    let now = time::OffsetDateTime::now_utc();
    let gateway_record = state.network_view().get_country_by_ip(&addr.ip()).await;

    let from_mixnet = gateway_record.is_some();
    let maybe_location = gateway_record.unwrap_or_default();

    if from_mixnet {
        debug!("Received a report from the network");
    } else {
        debug!("Received a report from outside of the network");
    }

    let active_device = DailyActiveDeviceDto::new(now, &report, user_agent, from_mixnet);
    let maybe_connection_info =
        ConnectionInfoDto::maybe_new(now, &report, addr, maybe_location, from_mixnet);

    state
        .storage()
        .store_vpn_client_report(active_device, maybe_connection_info)
        .await
        .map_err(HttpError::internal_with_logging)?;

    Ok(Json(()))
}
