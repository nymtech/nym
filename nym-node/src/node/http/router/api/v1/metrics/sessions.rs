// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::metrics::MetricsAppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_metrics::NymNodeMetrics;
use nym_node_requests::api::v1::metrics::models::{Session, SessionStats};
use time::macros::time;

/// If applicable, returns sessions statistics information of this node.
/// This information is **PURELY** self-reported and in no way validated.
#[utoipa::path(
    get,
    path = "/sessions",
    context_path = "/api/v1/metrics",
    tag = "Metrics",
    responses(
        (status = 200, content(
            (SessionStats = "application/json"),
            (SessionStats = "application/yaml")
        ))
    ),
    params(OutputParams),
)]
pub(crate) async fn sessions_stats(
    Query(output): Query<OutputParams>,
    State(metrics_state): State<MetricsAppState>,
) -> SessionStatsResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(build_response(&metrics_state.metrics).await)
}

async fn build_response(metrics: &NymNodeMetrics) -> SessionStats {
    let guard = metrics.entry.client_sessions().await;
    let sessions = guard
        .finished_sessions
        .iter()
        .map(|finished| Session {
            duration_ms: finished.duration.as_millis() as u64,
            typ: finished.typ.to_string(),
        })
        .collect();
    SessionStats {
        update_time: guard.update_time.with_time(time!(0:00)).assume_utc(),
        unique_active_users: guard.unique_users.len() as u32,
        unique_active_users_hashes: guard.unique_users.clone(),
        sessions,
        sessions_started: guard.sessions_started,
        sessions_finished: guard.finished_sessions.len() as u32,
    }
}

pub type SessionStatsResponse = FormattedResponse<SessionStats>;
