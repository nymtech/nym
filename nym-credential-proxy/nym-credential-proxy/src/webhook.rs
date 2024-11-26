// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::VpnApiError;
use clap::Args;
use reqwest::header::AUTHORIZATION;
use serde::Serialize;
use tracing::{debug, error, instrument, span, Level};
use url::Url;
use uuid::Uuid;

#[derive(Args, Debug, Clone)]
pub struct ZkNymWebHookConfig {
    #[clap(long, env = "WEBHOOK_ZK_NYMS_URL")]
    pub webhook_url: Url,

    #[clap(long, env = "WEBHOOK_ZK_NYMS_CLIENT_ID")]
    pub webhook_client_id: String,

    #[clap(long, env = "WEBHOOK_ZK_NYMS_CLIENT_SECRET")]
    pub webhook_client_secret: String,
}

impl ZkNymWebHookConfig {
    pub fn ensure_valid_client_url(&self) -> Result<(), VpnApiError> {
        self.client_url()
            .map_err(|_| VpnApiError::InvalidWebhookUrl)
            .map(|_| ())
    }

    fn client_url(&self) -> Result<Url, url::ParseError> {
        self.webhook_url.join(&self.webhook_client_id)
    }

    fn unchecked_client_url(&self) -> Url {
        // we ensured we have valid url on startup
        #[allow(clippy::unwrap_used)]
        self.client_url().unwrap()
    }

    fn bearer_token(&self) -> String {
        format!("Bearer {}", self.webhook_client_secret)
    }

    #[instrument(skip_all)]
    pub async fn try_trigger<T: Serialize + ?Sized>(&self, original_uuid: Uuid, payload: &T) {
        let url = self.unchecked_client_url();
        let span = span!(Level::DEBUG, "webhook", uuid = %original_uuid, url = %url);
        let _entered = span.enter();

        debug!("🕸️ about to trigger the webhook");

        match reqwest::Client::new()
            .post(url.clone())
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
}
