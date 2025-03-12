// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::models::status::{
    ActivePaymentWatchersResponse, ApiStatus, HealthResponse, PaymentListenerFailureDetails,
    PaymentListenerStatusResponse, ProcessedPayment, WatcherFailureDetails, WatcherState,
};
use crate::http::state::{AppState, PaymentListenerState, StatusState};
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use std::ops::Deref;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/build-information", get(build_information))
        .route("/active-payment-watchers", get(active_payment_watchers))
        .route("/payment-listener", get(payment_listener_status))
}

#[utoipa::path(
    tag = "Status",
    get,
    path = "/build-information",
    context_path = "/v1/status",
    responses(
        (status = 200, body = BinaryBuildInformationOwned)
    )
)]
async fn build_information(State(state): State<StatusState>) -> Json<BinaryBuildInformationOwned> {
    Json(state.build_information.to_owned())
}

#[utoipa::path(
    tag = "Status",
    get,
    path = "/health",
    context_path = "/v1/status",
    responses(
        (status = 200, body = HealthResponse)
    )
)]
async fn health(State(state): State<StatusState>) -> Json<HealthResponse> {
    let uptime = state.startup_time.elapsed();

    let health = HealthResponse {
        status: ApiStatus::Up,
        uptime: uptime.as_secs(),
    };
    Json(health)
}

#[utoipa::path(
    tag = "Status",
    get,
    path = "/active-payment-watchers",
    context_path = "/v1/status",
    responses(
        (status = 200, body = ActivePaymentWatchersResponse)
    )
)]
pub(crate) async fn active_payment_watchers(
    State(state): State<AppState>,
) -> Json<ActivePaymentWatchersResponse> {
    Json(ActivePaymentWatchersResponse {
        watchers: state.registered_payment_watchers.deref().clone(),
    })
}

#[utoipa::path(
    tag = "Status",
    get,
    path = "/payment-listener",
    context_path = "/v1/status",
    responses(
        (status = 200, body = PaymentListenerStatusResponse)
    )
)]
pub(crate) async fn payment_listener_status(
    State(state): State<PaymentListenerState>,
) -> Json<PaymentListenerStatusResponse> {
    let guard = state.inner.read().await;

    // sorry for the nasty conversion code here, run out of time : (
    Json(PaymentListenerStatusResponse {
        last_checked: guard.last_checked,
        processed_payments_since_startup: guard.processed_payments_since_startup,
        watcher_errors_since_startup: guard.watcher_errors_since_startup,
        payment_listener_errors_since_startup: guard.payment_listener_errors_since_startup,
        last_processed_payment: guard
            .last_processed_payment
            .as_ref()
            .map(|p| ProcessedPayment {
                processed_at: p.processed_at,
                tx_hash: p.tx_hash.clone(),
                message_index: p.message_index,
                height: p.height,
                sender: p.sender.clone(),
                receiver: p.receiver.clone(),
                funds: p.funds.clone(),
                memo: p.memo.clone(),
            }),
        latest_failures: guard
            .latest_failures
            .iter()
            .map(|f| PaymentListenerFailureDetails {
                timestamp: f.timestamp,
                error: f.error.clone(),
            })
            .collect(),
        watchers: guard
            .watchers
            .iter()
            .map(|(w, state)| {
                (
                    w.clone(),
                    WatcherState {
                        latest_failures: state
                            .latest_failures
                            .iter()
                            .map(|f| WatcherFailureDetails {
                                timestamp: f.timestamp,
                                error: f.error.clone(),
                            })
                            .collect(),
                    },
                )
            })
            .collect(),
    })
}
