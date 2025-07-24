// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::env::vars::*;
use crate::monitor::SignersMonitor;
use std::time::Duration;
use url::Url;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Specify email address for the bot responsible for sending notifications to the zulip server
    /// in case 'upgrade' mode is detected
    #[clap(
        long,
        env = ZULIP_BOT_EMAIL_ARG
    )]
    pub(crate) zulip_bot_email: String,

    /// Specify the API key for the bot responsible for sending notifications to the zulip server
    /// in case 'upgrade' mode is detected
    #[clap(
        long,
        env = ZULIP_BOT_API_KEY_ARG
    )]
    pub(crate) zulip_bot_api_key: String,

    /// Specify the sever endpoint for the bot responsible for sending notifications
    /// in case 'upgrade' mode is detected
    #[clap(
        long,
        env = ZULIP_SERVER_URL_ARG
    )]
    pub(crate) zulip_server_url: Url,

    /// Specify the channel id for where the notification is going to be sent
    #[clap(
        long,
        env = ZULIP_NOTIFICATION_CHANNEL_ID_ARG
    )]
    pub(crate) zulip_notification_channel_id: u32,

    /// Specify the delay between subsequent signers checks
    #[clap(
        long,
        env = SIGNERS_MONITOR_CHECK_INTERVAL_ARG,
        value_parser = humantime::parse_duration,
        default_value = "15m"
    )]
    pub(crate) signers_check_interval: Duration,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    SignersMonitor::new(args)?.run().await
}
