// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::run;
use crate::test_result::{DisplayableSignerResult, Summary, TestResult};
use nym_ecash_signer_check::check_signers_with_client;
use nym_task::ShutdownManager;
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info};

pub(crate) struct SignersMonitor {
    zulip_client: zulip_client::Client,
    nyxd_client: QueryHttpRpcNyxdClient,

    notification_channel_id: u32,
    notification_topic: Option<String>,
    check_interval: Duration,
    // min_notification_delay: Duration,
    // last_notification_sent: Option<OffsetDateTime>,
}

impl SignersMonitor {
    pub(crate) fn new(args: run::Args) -> anyhow::Result<Self> {
        let zulip_client = zulip_client::Client::builder(
            args.zulip_bot_email,
            args.zulip_bot_api_key,
            args.zulip_server_url,
        )?
        .build()?;
        let nyxd_client = args.nyxd_connection.try_create_nyxd_client()?;

        Ok(SignersMonitor {
            zulip_client,
            nyxd_client,
            notification_channel_id: args.zulip_notification_channel_id,
            notification_topic: args.zulip_notification_topic,
            check_interval: args.signers_check_interval,
        })
    }

    async fn check_signers(&self) -> anyhow::Result<TestResult> {
        info!("starting signer check...");
        let check_result = check_signers_with_client(&self.nyxd_client).await?;

        let mut unreachable_signers = 0;
        let mut unknown_local_chain_status = 0;
        let mut stalled_local_chain = 0;
        let mut working_local_chain = 0;
        let mut unknown_credential_issuance_status = 0;
        let mut working_credential_issuance = 0;
        let mut unavailable_credential_issuance = 0;

        let mut fully_working = 0;

        let mut signers = Vec::new();
        for result in &check_result.results {
            if result.signer_unreachable() {
                unreachable_signers += 1;
            }

            if result.unknown_chain_status() {
                unknown_local_chain_status += 1;
            }
            if result.chain_available() {
                working_local_chain += 1;
            }
            if result.chain_provably_stalled() || result.chain_unprovably_stalled() {
                stalled_local_chain += 1;
            }

            if result.unknown_signing_status() {
                unknown_credential_issuance_status += 1;
            }
            if result.signing_available() {
                working_credential_issuance += 1;
            }
            if result.signing_provably_unavailable() || result.signing_unprovably_unavailable() {
                unavailable_credential_issuance += 1;
            }

            let signing_available = if result.unknown_signing_status() {
                None
            } else {
                Some(result.signing_available())
            };

            let chain_not_stalled = if result.unknown_chain_status() {
                None
            } else {
                Some(result.chain_available())
            };

            if (result.chain_available()) && (result.signing_available()) {
                fully_working += 1;
            }

            signers.push(DisplayableSignerResult {
                version: result
                    .try_get_test_result()
                    .map(|r| r.reported_version.clone()),
                url: result.information.announce_address.clone(),
                signing_available,
                chain_not_stalled,
            })
        }

        let signing_quorum_available = check_result.threshold.map(|threshold| {
            (working_local_chain as u64) >= threshold
                && (working_credential_issuance as u64) >= threshold
        });
        signers.sort_by_key(|s| s.version);

        let summary = Summary {
            signing_quorum_available,
            fully_working,
            unreachable_signers,
            registered_signers: check_result.results.len(),
            unknown_local_chain_status,
            stalled_local_chain,
            working_local_chain,
            unknown_credential_issuance_status,
            working_credential_issuance,
            unavailable_credential_issuance,
            threshold: check_result.threshold,
        };

        Ok(TestResult { summary, signers })
    }

    async fn perform_signer_check(&self) -> anyhow::Result<()> {
        let result = self.check_signers().await?;

        if result.quorum_unavailable() {
            let message = format!(
                r#"
# 🔥🔥🔥 LOST SIGNING QUORUM 🔥🔥🔥
We seem to have lost the signing quorum - check if we should enable the 'upgrade' mode!

{}
            "#,
                result.results_to_markdown_message()
            );
            return self.send_zulip_notification(&message).await;
        }

        if result.quorum_unknown() {
            let message = format!(
                r#"
# ❓❓❓ UNKNOWN SIGNING QUORUM ❓❓❓
We can't determine the signing quroum - if we're not undergoing DKG exchange check if we should enable the 'upgrade' mode!

{}
            "#,
                result.results_to_markdown_message()
            );
            return self.send_zulip_notification(&message).await;
        }

        Ok(())
    }

    async fn send_zulip_notification(&self, message: &String) -> anyhow::Result<()> {
        self.zulip_client
            .send_channel_message((
                self.notification_channel_id,
                message,
                &self.notification_topic,
            ))
            .await?;
        Ok(())
    }

    async fn send_shutdown_notification(&self) -> anyhow::Result<()> {
        println!("here be sending shutdown notification");
        Ok(())
    }

    pub(crate) async fn run(&mut self) -> anyhow::Result<()> {
        let shutdown_manager =
            ShutdownManager::new("nym-signers-monitor").with_default_shutdown_signals()?;

        let mut check_interval = interval(self.check_interval);

        while !shutdown_manager.root_token.is_cancelled() {
            tokio::select! {
                biased;
                _ = shutdown_manager.root_token.cancelled() => {
                    info!("received shutdown");
                    break;
                }
                _ = check_interval.tick() => {
                    if let Err(err) = self.perform_signer_check().await {
                        error!("failed to check signers: {err}");
                    }
                }

            }
        }

        shutdown_manager.close();
        shutdown_manager.run_until_shutdown().await;

        if let Err(err) = self.send_shutdown_notification().await {
            error!("failed to send shutdown notification: {err}");
        }

        Ok(())
    }
}
