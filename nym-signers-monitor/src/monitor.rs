// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::run;
use nym_ecash_signer_check::check_signers;
use nym_task::ShutdownManager;
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::interval;
use tracing::{error, info};

pub(crate) struct SignersMonitor {
    zulip_client: zulip_client::Client,
    nyxd_client: QueryHttpRpcNyxdClient,
    check_interval: Duration,
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
            check_interval: args.signers_check_interval,
        })
    }

    async fn check_signers(&self) -> anyhow::Result<()> {
        println!("here be checking the signers");
        Ok(())
    }

    async fn send_shutdown_notification(&self) -> anyhow::Result<()> {
        println!("here be sending shutdown notification");
        Ok(())
    }

    pub(crate) async fn run(&mut self) -> anyhow::Result<()> {
        let mut shutdown_manager =
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
                    if let Err(err) = self.check_signers().await {
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
