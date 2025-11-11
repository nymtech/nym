// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::data_observatory::{HttpAuthenticationOptions, WebhookConfig};
use crate::models::WebhookPayload;
use anyhow::Context;
use async_trait::async_trait;
use nyxd_scraper_psql::{
    NyxdScraperTransaction, ParsedTransactionResponse, ScraperError, TxModule,
};
use reqwest::{Client, Url};
use tracing::{error, info};
use utoipa::r#gen::serde_json;

pub struct WebhookModule {
    webhooks: Vec<Webhook>,
}

impl WebhookModule {
    pub fn new(config: crate::config::Config) -> anyhow::Result<Self> {
        let webhooks = config
            .data_observatory_config
            .webhooks
            .iter()
            .map(|watcher_cfg| Webhook::new(watcher_cfg.clone()))
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Self { webhooks })
    }
}

#[async_trait]
impl TxModule for WebhookModule {
    async fn handle_tx(
        &mut self,
        tx: &ParsedTransactionResponse,
        _: &mut dyn NyxdScraperTransaction,
    ) -> Result<(), ScraperError> {
        for (index, msg) in &tx.parsed_messages {
            if let Some(parsed_message_type_url) = tx.parsed_message_urls.get(&index) {
                let payload = WebhookPayload {
                    height: tx.height.value(),
                    message_index: index.clone() as u64,
                    transaction_hash: tx.hash.to_string(),
                    message: Some(msg.clone()),
                };

                // println!(
                //     "->>>>>>>>>>>>>>>>>>>>>>>>> {}",
                //     serde_json::to_string(&payload).unwrap()
                // );

                for webhook in self.webhooks.clone() {
                    // if the webhook requires a type and the parsed message type doesn't match, skip
                    if !webhook.config.watch_for_chain_message_types.is_empty()
                        && !webhook
                            .config
                            .watch_for_chain_message_types
                            .contains(parsed_message_type_url)
                    {
                        continue;
                    }

                    let payload = payload.clone();

                    // TODO: some excellent advice from Andrew, for another day:
                    //   - pass a cancellation token for shutdown
                    //   - use TaskManager and limit number of webhooks to spawn at once
                    tokio::spawn(async move {
                        if let Err(e) = webhook.invoke_webhook(&payload).await {
                            error!("webhook error: {}", e);
                        }
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct Webhook {
    webhook_url: Url,
    config: WebhookConfig,
}

impl Webhook {
    pub(crate) fn new(config: WebhookConfig) -> anyhow::Result<Self> {
        Ok(Webhook {
            webhook_url: config
                .webhook_url
                .as_str()
                .parse()
                .context("invalid config: provided webhook URL is malformed")?,
            config,
        })
    }

    pub(crate) fn id(&self) -> &str {
        &self.config.id
    }

    pub(crate) async fn invoke_webhook(&self, payload: &WebhookPayload) -> anyhow::Result<()> {
        let client = Client::builder()
            .user_agent(format!(
                "nym-data-observatory/{}/webhook-{}",
                env!("CARGO_PKG_VERSION"),
                self.id()
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
                "[webhook = {}] ✅ Webhook {} {} - tx {}, index {}",
                self.config.id,
                res.status(),
                res.url(),
                payload.transaction_hash,
                payload.message_index,
            ),
            Err(err) => {
                error!(
                    "[webhook = {}] ❌ Webhook {:?} {:?} error = {err}",
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
