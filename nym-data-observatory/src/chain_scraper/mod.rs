use crate::cli::commands::run::Args;
use crate::db::DbPool;
use nyxd_scraper_psql::{PostgresNyxdScraper, PruningOptions};
use std::fs;
use tracing::{info, warn};

pub(crate) mod webhook;

pub(crate) async fn run_chain_scraper(
    args: Args,
    config: &crate::config::Config,
    connection_pool: DbPool,
) -> anyhow::Result<PostgresNyxdScraper> {
    let use_best_effort_start_height = args.start_block_height.is_some();

    if args.nuke_db {
        warn!("☢️☢️☢️ NUKING THE SCRAPER DATABASE");
        fs::remove_file(config.chain_scraper_connection_string())?;
    }

    let database_storage = config
        .chain_scraper_connection_string
        .clone()
        .and(args.db_connection_string)
        .expect("no database connection string set in config");

    let scraper = PostgresNyxdScraper::builder(nyxd_scraper_psql::Config {
        websocket_url: args.websocket_url,
        rpc_url: args.rpc_url,
        database_storage,
        pruning_options: PruningOptions::nothing(),
        store_precommits: false,
        start_block: nyxd_scraper_psql::StartingBlockOpts {
            start_block_height: args.start_block_height,
            use_best_effort_start_height,
        },
    })
    .with_msg_module(crate::modules::wasm::WasmModule::new(connection_pool))
    .with_msg_module(webhook::WebhookModule::new(config.clone())?);

    let instance = scraper.build_and_start().await?;

    info!("🚧 blocking until the chain has caught up...");
    instance.wait_for_startup_sync().await;

    Ok(instance)
}
