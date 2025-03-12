// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// if we ever create some sort of chain watcher client, those would need to be extracted

pub mod status {
    use crate::config::payments_watcher::PaymentWatcherConfig;
    use crate::db::models::CoingeckoPriceResponse;
    use crate::models::openapi_schema;
    use nym_validator_client::nyxd::Coin;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use time::OffsetDateTime;
    use utoipa::ToSchema;

    #[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
    #[serde(rename_all = "lowercase")]
    pub enum ApiStatus {
        Up,
    }

    #[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
    pub struct HealthResponse {
        pub status: ApiStatus,
        pub uptime: u64,
    }

    #[derive(Debug, Serialize, Deserialize, ToSchema)]
    pub struct ActivePaymentWatchersResponse {
        pub watchers: Vec<PaymentWatcher>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct PaymentWatcher {
        pub id: String,
        pub description: String,
        pub webhook_url: String,
        pub watched_accounts: Vec<String>,
        pub watched_message_types: Vec<String>,
    }

    impl From<&PaymentWatcherConfig> for PaymentWatcher {
        fn from(value: &PaymentWatcherConfig) -> Self {
            PaymentWatcher {
                id: value.id.clone(),
                description: value.description.clone().unwrap_or_default(),
                webhook_url: value.webhook_url.clone(),
                watched_accounts: value
                    .watch_for_transfer_recipient_accounts
                    .iter()
                    .map(|a| a.to_string())
                    .collect(),
                watched_message_types: value.watch_for_chain_message_types.clone(),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct PaymentListenerStatusResponse {
        #[serde(with = "time::serde::rfc3339")]
        pub last_checked: OffsetDateTime,

        pub processed_payments_since_startup: u64,
        pub watcher_errors_since_startup: u64,
        pub payment_listener_errors_since_startup: u64,

        pub last_processed_payment: Option<ProcessedPayment>,

        pub latest_failures: Vec<PaymentListenerFailureDetails>,
        pub watchers: HashMap<String, WatcherState>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub(crate) struct ProcessedPayment {
        #[serde(with = "time::serde::rfc3339")]
        pub processed_at: OffsetDateTime,

        pub tx_hash: String,
        pub message_index: u64,
        pub height: u64,
        pub sender: String,
        pub receiver: String,

        #[schema(value_type = openapi_schema::Coin)]
        pub funds: Coin,

        pub memo: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub(crate) struct PaymentListenerFailureDetails {
        #[serde(with = "time::serde::rfc3339")]
        pub(crate) timestamp: OffsetDateTime,
        pub(crate) error: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub(crate) struct WatcherState {
        pub(crate) latest_failures: Vec<WatcherFailureDetails>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub(crate) struct WatcherFailureDetails {
        #[serde(with = "time::serde::rfc3339")]
        pub(crate) timestamp: OffsetDateTime,
        pub(crate) error: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub(crate) struct PriceScraperStatusResponse {
        pub(crate) last_success: Option<PriceScraperLastSuccess>,
        pub(crate) last_failure: Option<PriceScraperLastError>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub(crate) struct PriceScraperLastSuccess {
        #[serde(with = "time::serde::rfc3339")]
        pub(crate) timestamp: OffsetDateTime,
        pub(crate) response: CoingeckoPriceResponse,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub(crate) struct PriceScraperLastError {
        #[serde(with = "time::serde::rfc3339")]
        pub(crate) timestamp: OffsetDateTime,
        pub(crate) message: String,
    }
}
