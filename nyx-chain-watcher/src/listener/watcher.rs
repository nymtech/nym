// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::payments_watcher::{HttpAuthenticationOptions, PaymentWatcherConfig};
use crate::models::WebhookPayload;
use anyhow::Context;
use reqwest::{Client, Url};
use tracing::{error, info};

pub(crate) struct PaymentWatcher {
    webhook_url: Url,
    config: PaymentWatcherConfig,
}

impl PaymentWatcher {
    pub(crate) fn new(config: PaymentWatcherConfig) -> anyhow::Result<Self> {
        Ok(PaymentWatcher {
            webhook_url: config
                .webhook_url
                .as_str()
                .parse()
                .context("couldn't create payment watcher: provided webhook URL is malformed")?,
            config,
        })
    }

    pub(super) fn id(&self) -> &str {
        &self.config.id
    }

    pub(crate) async fn invoke_webhook(&self, payload: &WebhookPayload) -> anyhow::Result<()> {
        let client = Client::builder()
            .user_agent(format!(
                "nyx-chain-watcher/{}/watcher-{}",
                env!("CARGO_PKG_VERSION"),
                self.config.id
            ))
            .build()
            .context("failed to build reqwest client")?;

        let mut request_builder = client.post(self.webhook_url.clone()).json(payload);

        if let Some(auth) = &self.config.authentication {
            match auth {
                HttpAuthenticationOptions::AuthorizationBearerToken { token } => {
                    request_builder = request_builder.bearer_auth(token);
                }
            }
        }

        match request_builder.send().await {
            Ok(res) => info!(
                "[watcher = {}] ✅ Webhook {} {} - tx {}, index {}",
                self.config.id,
                res.status(),
                res.url(),
                payload.transaction_hash,
                payload.message_index,
            ),
            Err(err) => {
                error!(
                    "[watcher = {}] ❌ Webhook {:?} {:?} error = {err}",
                    self.config.id,
                    err.status(),
                    err.url(),
                );
                return Err(err.into());
            }
        }

        Ok(())
    }
}
