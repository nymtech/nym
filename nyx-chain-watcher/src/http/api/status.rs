// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::models::status::{
    ActivePaymentWatchersResponse, ApiStatus, BankModuleStatusResponse, BankMsgDetails,
    BankMsgRejection, HealthResponse, PaymentListenerFailureDetails, PaymentListenerStatusResponse,
    PriceScraperLastError, PriceScraperLastSuccess, PriceScraperStatusResponse, ProcessedPayment,
    WatcherFailureDetails, WatcherState,
};
use crate::http::state::{
    AppState, BankScraperModuleState, PaymentListenerState, PriceScraperState, StatusState,
};
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
        .route("/price-scraper", get(price_scraper_status))
        .route("/bank-module-scraper", get(bank_module_status))
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

#[utoipa::path(
    tag = "Status",
    get,
    path = "/price-scraper",
    context_path = "/v1/status",
    responses(
        (status = 200, body = PriceScraperStatusResponse)
    )
)]
pub(crate) async fn price_scraper_status(
    State(state): State<PriceScraperState>,
) -> Json<PriceScraperStatusResponse> {
    let guard = state.inner.read().await;
    Json(PriceScraperStatusResponse {
        last_success: guard
            .last_success
            .as_ref()
            .map(|s| PriceScraperLastSuccess {
                timestamp: s.timestamp,
                response: s.response.clone(),
            }),
        last_failure: guard.last_failure.as_ref().map(|f| PriceScraperLastError {
            timestamp: f.timestamp,
            message: f.message.clone(),
        }),
    })
}

#[utoipa::path(
    tag = "Status",
    get,
    path = "/bank-module-scraper",
    context_path = "/v1/status",
    responses(
        (status = 200, body = BankModuleStatusResponse)
    )
)]
pub(crate) async fn bank_module_status(
    State(state): State<BankScraperModuleState>,
) -> Json<BankModuleStatusResponse> {
    let guard = state.inner.read().await;
    Json(BankModuleStatusResponse {
        processed_bank_msgs_since_startup: guard.processed_bank_msgs_since_startup,
        processed_bank_msgs_to_watched_addresses_since_startup: guard
            .processed_bank_msgs_to_watched_addresses_since_startup,
        rejected_bank_msgs_to_watched_addresses_since_startup: guard
            .rejected_bank_msgs_to_watched_addresses_since_startup,
        last_seen_bank_msgs: guard
            .last_seen_bank_msgs
            .iter()
            .map(|msg| BankMsgDetails {
                processed_at: msg.processed_at,
                tx_hash: msg.tx_hash.clone(),
                height: msg.height,
                index: msg.index,
                from: msg.from.clone(),
                to: msg.to.clone(),
                amount: msg.amount.clone(),
                memo: msg.memo.clone(),
            })
            .collect(),
        last_seen_watched_bank_msgs: guard
            .last_seen_watched_bank_msgs
            .iter()
            .map(|msg| BankMsgDetails {
                processed_at: msg.processed_at,
                tx_hash: msg.tx_hash.clone(),
                height: msg.height,
                index: msg.index,
                from: msg.from.clone(),
                to: msg.to.clone(),
                amount: msg.amount.clone(),
                memo: msg.memo.clone(),
            })
            .collect(),
        last_rejected_watched_bank_msgs: guard
            .last_rejected_watched_bank_msgs
            .iter()
            .map(|r| BankMsgRejection {
                rejected_at: r.rejected_at,
                tx_hash: r.tx_hash.clone(),
                height: r.height,
                index: r.index,
                error: r.error.clone(),
            })
            .collect(),
    })
}
