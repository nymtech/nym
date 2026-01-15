use axum::{Json, Router, extract::State};
use axum_client_ip::InsecureClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};
use nym_statistics_common::report::vpn_client::{VpnClientStatsReport, VpnClientStatsReportV2};
use tracing::debug;

use crate::{
    http::{
        error::{HttpError, HttpResult},
        state::AppState,
    },
    storage::models::{DailyActiveDeviceDto, StatsReportV1Dto, StatsReportV2Dto},
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/report", axum::routing::post(submit_stats_report))
        .route("/session", axum::routing::post(submit_session_report))
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
    Json(report): Json<VpnClientStatsReport>,
) -> HttpResult<Json<()>> {
    let now = time::OffsetDateTime::now_utc();

    let gateway_record = state
        .network_view()
        .get_country_by_ip(&insecure_ip_addr.0)
        .await;

    let from_mixnet = gateway_record.is_some();
    let maybe_location = gateway_record.unwrap_or_default();

    if from_mixnet {
        debug!("Received a V1 report from the network");
    } else {
        debug!("Received a V1 report from outside of the network");
    }
    let active_device = DailyActiveDeviceDto::new(now, &report, user_agent.clone(), from_mixnet);

    let stats_report = StatsReportV1Dto::new(
        now,
        &report,
        user_agent,
        from_mixnet,
        insecure_ip_addr.0,
        maybe_location,
    );

    state
        .storage()
        .store_vpn_client_report(stats_report)
        .await
        .map_err(HttpError::internal_with_logging)?;

    state
        .storage()
        .store_active_device(active_device)
        .await
        .map_err(HttpError::internal_with_logging)?;

    Ok(Json(()))
}

#[utoipa::path(
    post,
    request_body = VpnClientStatsReportV2,
    tag = "Stats",
    path = "/session",
    context_path = "/v1/stats",
    responses(
        (status = 200)
    )
)]
#[tracing::instrument(level = "info", skip_all)]
async fn submit_session_report(
    State(mut state): State<AppState>,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    insecure_ip_addr: InsecureClientIp, // This is the reverse proxy IP for now, but maybe in the future?
    Json(report): Json<VpnClientStatsReportV2>,
) -> HttpResult<Json<()>> {
    let now = time::OffsetDateTime::now_utc();
    let gateway_record = state
        .network_view()
        .get_country_by_id(&report.session_report.exit_id)
        .await;

    let from_mixnet = gateway_record.is_some();
    let maybe_location = gateway_record.unwrap_or_default();

    if from_mixnet {
        debug!("Received a V2 report from the network");
    } else {
        debug!("Received a V2 report from outside of the network");
    }
    let active_device = DailyActiveDeviceDto::new_v2(now, &report, user_agent.clone(), from_mixnet);

    let stats_report = StatsReportV2Dto::new(
        now,
        &report,
        user_agent,
        from_mixnet,
        insecure_ip_addr.0,
        maybe_location,
    );

    state
        .storage()
        .store_active_device(active_device)
        .await
        .map_err(HttpError::internal_with_logging)?;

    state
        .storage()
        .store_vpn_client_report_v2(stats_report)
        .await
        .map_err(HttpError::internal_with_logging)?;

    Ok(Json(()))
}
