use axum::{extract::State, http::HeaderMap, Json, Router};
use axum_client_ip::InsecureClientIp;
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
    tag = "Stats",
    path = "/report",
    context_path = "/v1/stats",
    responses(
        (status = 200)
    )
)]
#[tracing::instrument(level = "info", skip_all)]
async fn submit_stats_report(
    State(mut state): State<AppState>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    insecure_ip_addr: InsecureClientIp,
    headers: HeaderMap,
    Json(report): Json<VpnClientStatsReport>,
) -> HttpResult<Json<()>> {
    let now = time::OffsetDateTime::now_utc();

    if let Some(h) = headers.get("X-Real-IP") {
        debug!("X-Real-IP :{:?}", h);
    }
    if let Some(h) = headers.get("X-Forwarded-For") {
        debug!("X-Forwarded-For :{:?}", h);
    }

    let gateway_record = state
        .network_view()
        .get_country_by_ip(&insecure_ip_addr.0)
        .await;

    let from_mixnet = gateway_record.is_some();
    let maybe_location = gateway_record.unwrap_or_default();

    if from_mixnet {
        debug!("Received a report from the network");
    } else {
        debug!("Received a report from outside of the network");
    }

    let active_device = DailyActiveDeviceDto::new(now, &report, user_agent, from_mixnet);
    let maybe_connection_info = ConnectionInfoDto::maybe_new(
        now,
        &report,
        insecure_ip_addr.0.to_string(),
        maybe_location,
        from_mixnet,
    );

    state
        .storage()
        .store_vpn_client_report(active_device, maybe_connection_info)
        .await
        .map_err(HttpError::internal_with_logging)?;

    Ok(Json(()))
}
