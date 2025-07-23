// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::data_observatory::{HttpAuthenticationOptions, WebhookConfig};
use crate::models::WebhookPayload;
use anyhow::Context;
use async_trait::async_trait;
use nym_validator_client::nyxd::{Any, Msg, MsgSend, Name};
use nyxd_scraper_psql::{
    MsgModule, NyxdScraperTransaction, ParsedTransactionResponse, ScraperError,
};
use reqwest::{Client, Url};
use tracing::{error, info};
use utoipa::gen::serde_json;

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
impl MsgModule for WebhookModule {
    fn type_url(&self) -> String {
        <MsgSend as Msg>::Proto::type_url()
    }

    async fn handle_msg(
        &mut self,
        index: usize,
        _msg: &Any,
        tx: &ParsedTransactionResponse,
        _storage_tx: &mut dyn NyxdScraperTransaction,
    ) -> Result<(), ScraperError> {
        let message = serde_json::to_value(tx.parsed_messages.get(&index)).ok();

        let payload = WebhookPayload {
            height: tx.height.value(),
            message_index: index as u64,
            transaction_hash: tx.hash.to_string(),
            message,
        };

        println!(
            "->>>>>>>>>>>>>>>>>>>>>>>>> {}",
            serde_json::to_string(&payload).unwrap()
        );

        for webhook in self.webhooks.clone() {
            let payload = payload.clone();
            tokio::spawn(async move {
                webhook
                    .invoke_webhook(&payload)
                    .await
                    .expect("webhook failed to process");
            });
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
