use crate::env::vars::{NYXD_SCRAPER_START_HEIGHT, NYXD_SCRAPER_USE_BEST_EFFORT_START_HEIGHT};
use nyxd_scraper::{storage::ScraperStorage, NyxdScraper, PruningOptions};

pub(crate) async fn run_chain_scraper(
    config: &crate::config::Config,
) -> anyhow::Result<ScraperStorage> {
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

    let scraper = NyxdScraper::builder(nyxd_scraper::Config {
        websocket_url,
        rpc_url,
        database_path: config.chain_scraper_database_path().into(),
        pruning_options: PruningOptions::nothing(),
        store_precommits: false,
        start_block: nyxd_scraper::StartingBlockOpts {
            start_block_height,
            use_best_effort_start_height,
        },
    });

    let instance = scraper.build_and_start().await?;

    Ok(instance.storage)
}
