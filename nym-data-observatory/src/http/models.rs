// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// if we ever create some sort of chain watcher client, those would need to be extracted

pub mod status {
    use crate::config::data_observatory::WebhookConfig;
    use crate::db::models::CoingeckoPriceResponse;
    use serde::{Deserialize, Serialize};
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
    pub struct ActiveWebhooksResponse {
        pub watchers: Vec<Webhook>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct Webhook {
        pub id: String,
        pub description: String,
        pub webhook_url: String,
        pub watched_message_types: Vec<String>,
    }

    impl From<&WebhookConfig> for Webhook {
        fn from(value: &WebhookConfig) -> Self {
            Webhook {
                id: value.id.clone(),
                description: value.description.clone().unwrap_or_default(),
                webhook_url: value.webhook_url.clone(),
                watched_message_types: value.watch_for_chain_message_types.clone(),
            }
        }
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
