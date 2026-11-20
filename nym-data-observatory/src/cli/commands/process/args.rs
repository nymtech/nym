// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::env::vars::*;
use url::Url;

#[derive(clap::Args, Debug, Clone)]
pub(crate) struct Args {
    #[arg(long, alias = "start")]
    pub(crate) start_block_height: u32,

    #[arg(long, alias = "end")]
    pub(crate) end_block_height: Option<u32>,

    #[arg(long, alias = "blocks")]
    pub(crate) blocks_to_process: Option<u32>,

    #[arg(long, env = NYM_DATA_OBSERVATORY_DB_URL, alias = "db_url")]
    pub(crate) db_connection_string: Option<String>,

    #[arg(long, env = NYXD_WS, alias = "nyxd_ws", default_value = "wss://rpc.nymtech.net/websocket")]
    pub(crate) websocket_url: Url,

    #[arg(long, env = NYXD, alias = "nyxd", default_value = "https://rpc.nymtech.net")]
    pub(crate) rpc_url: Url,

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
