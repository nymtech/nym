// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use reqwest::header::AUTHORIZATION;
use serde::Serialize;
use tracing::{Instrument, Level, debug, error, instrument, span};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ZkNymWebhook {
    pub webhook_client_url: Url,

    pub webhook_client_secret: String,
}

impl ZkNymWebhook {
    fn bearer_token(&self) -> String {
        format!("Bearer {}", self.webhook_client_secret)
    }

    #[instrument(skip_all)]
    pub async fn try_trigger<T: Serialize + ?Sized>(&self, original_uuid: Uuid, payload: &T) {
        let url = self.webhook_client_url.clone();
        let span = span!(Level::DEBUG, "webhook", uuid = %original_uuid, url = %url);

        async move {
            debug!("🕸️ about to trigger the webhook");

            match reqwest::Client::new()
                .post(url)
                .header(AUTHORIZATION, self.bearer_token())
                .json(payload)
                .send()
                .await
            {
                Ok(res) => {
                    if !res.status().is_success() {
                        error!("❌🕸️ failed to call webhook: {res:?}");
                    } else {
                        debug!("✅🕸️ webhook triggered successfully: {res:?}");
                        if let Ok(body) = res.text().await {
                            debug!("body = {body}");
                        }
                    }
                }
                Err(err) => {
                    error!("failed to call webhook: {err}")
                }
            }
        }
        .instrument(span)
        .await
    }
}
