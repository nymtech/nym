use axum::{Json, Router, extract::State};
use axum_client_ip::InsecureClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};
use nym_statistics_common::report::vpn_client::{VpnClientStatsReport, VpnSessionReport};
use tracing::debug;

use crate::{
    http::{
        error::{HttpError, HttpResult},
        state::AppState,
    },
    storage::models::{ConnectionInfoDto, DailyActiveDeviceDto, SessionInfoDto, StatsReportV1Dto},
};
pub(crate) fn routes() -> Router<AppState> {}
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
        debug!("Received a report from the network");
    } else {
        debug!("Received a report from outside of the network");
    }

    let active_device = DailyActiveDeviceDto::new(now, &report, user_agent.clone(), from_mixnet);
    let maybe_connection_info = ConnectionInfoDto::maybe_new(
        now,
        &report,
        insecure_ip_addr.0,
        maybe_location,
        from_mixnet,
    );

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
        .store_legacy_vpn_client_report(active_device, maybe_connection_info)
        .await
        .map_err(HttpError::internal_with_logging)?;

    Ok(Json(()))
}

#[utoipa::path(
    post,
    request_body = VpnSessionReport,
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
    Json(report): Json<VpnSessionReport>,
) -> HttpResult<Json<()>> {
    let now = time::OffsetDateTime::now_utc();
    let gateway_record = state
        .network_view()
        .get_country_by_id(&report.exit_id)
        .await;

    let from_mixnet = gateway_record.is_some();
    let maybe_location = gateway_record.unwrap_or_default();

    let session_info = SessionInfoDto::new(now, &report, maybe_location, from_mixnet);

    state
        .storage()
        .store_vpn_session_report(session_info)
        .await
        .map_err(HttpError::internal_with_logging)?;

    Ok(Json(()))
}
