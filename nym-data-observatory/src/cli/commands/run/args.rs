// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::env::vars::*;
use url::Url;

#[derive(clap::Args, Debug, Clone)]
pub(crate) struct Args {
    #[arg(long, env = NYXD_WS, alias = "nyxd_ws")]
    pub(crate) websocket_url: Url,

    #[arg(long, env = NYXD, alias = "nyxd")]
    pub(crate) rpc_url: Url,

    #[arg(long, env = NYXD_SCRAPER_START_HEIGHT)]
    pub(crate) start_block_height: Option<u32>,

    /// (Override) Postgres connection string for chain scraper history
    #[arg(long, env = NYM_DATA_OBSERVATORY_DB_URL, alias = "db_url")]
    pub(crate) db_connection_string: Option<String>,

    /// (Override) Watch for chain messages of these types
    #[clap(
        long,
        value_delimiter = ',',
        env = NYM_DATA_OBSERVATORY_WATCH_CHAIN_MESSAGE_TYPES
    )]
    pub watch_for_chain_message_types: Vec<String>,

    /// (Override) The webhook to call when we find something
    #[clap(
        long,
        env = NYM_DATA_OBSERVATORY_WEBHOOK_URL
    )]
    pub webhook_url: Option<Url>,

    /// (Override) Optionally, authenticate with the webhook
    #[clap(
        long,
        env = NYM_DATA_OBSERVATORY_WEBHOOK_AUTH
    )]
    pub webhook_auth: Option<String>,
}
