use crate::db::DbPool;
use crate::env::vars::{
    NYXD_SCRAPER_START_HEIGHT, NYXD_SCRAPER_UNSAFE_NUKE_DB,
    NYXD_SCRAPER_USE_BEST_EFFORT_START_HEIGHT,
};
use nyxd_scraper_psql::{PostgresNyxdScraper, PruningOptions};
use std::fs;
use tracing::{info, warn};

pub(crate) mod webhook;

pub(crate) async fn run_chain_scraper(
    config: &crate::config::Config,
    _connection_pool: DbPool,
) -> anyhow::Result<PostgresNyxdScraper> {
    let websocket_url = std::env::var("NYXD_WS").expect("NYXD_WS not defined");

    let rpc_url = std::env::var("NYXD").expect("NYXD not defined");
    let websocket_url = reqwest::Url::parse(&websocket_url)?;
    let rpc_url = reqwest::Url::parse(&rpc_url)?;

    // why are those not part of CLI? : (
    let start_block_height = match std::env::var(NYXD_SCRAPER_START_HEIGHT).ok() {
        None => None,
        // blow up if passed malformed env value
        Some(raw) => Some(raw.parse()?),
    };

    let use_best_effort_start_height =
        match std::env::var(NYXD_SCRAPER_USE_BEST_EFFORT_START_HEIGHT).ok() {
            None => false,
            // blow up if passed malformed env value
            Some(raw) => raw.parse()?,
        };

    let nuke_db: bool = match std::env::var(NYXD_SCRAPER_UNSAFE_NUKE_DB).ok() {
        None => false,
        // blow up if passed malformed env value
        Some(raw) => raw.parse()?,
    };

    if nuke_db {
        warn!("☢️☢️☢️ NUKING THE SCRAPER DATABASE");
        fs::remove_file(config.chain_scraper_connection_string())?;
    }

    let scraper = PostgresNyxdScraper::builder(nyxd_scraper_psql::Config {
        websocket_url,
        rpc_url,
        database_storage: config.chain_scraper_connection_string.clone(),
        pruning_options: PruningOptions::nothing(),
        store_precommits: false,
        start_block: nyxd_scraper_psql::StartingBlockOpts {
            start_block_height,
            use_best_effort_start_height,
        },
    })
    .with_msg_module(webhook::WebhookModule::new(config.clone())?);

    let instance = scraper.build_and_start().await?;

    info!("🚧 blocking until the chain has caught up...");
    instance.wait_for_startup_sync().await;

    Ok(instance)
}
