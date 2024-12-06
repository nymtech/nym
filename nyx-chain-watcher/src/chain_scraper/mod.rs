use nyxd_scraper::{storage::ScraperStorage, NyxdScraper, PruningOptions};

pub(crate) async fn run_chain_scraper(
    config: &crate::config::Config,
) -> anyhow::Result<ScraperStorage> {
    let websocket_url = std::env::var("NYXD_WS").expect("NYXD_WS not defined");

    let rpc_url = std::env::var("NYXD").expect("NYXD not defined");
    let websocket_url = reqwest::Url::parse(&websocket_url)?;
    let rpc_url = reqwest::Url::parse(&rpc_url)?;

    let start_block_height = std::env::var("NYXD_SCRAPER_START_HEIGHT")
        .ok()
        .and_then(|value| value.parse::<u32>().ok());

    let scraper = NyxdScraper::builder(nyxd_scraper::Config {
        websocket_url,
        rpc_url,
        database_path: config.chain_scraper_database_path().into(),
        pruning_options: PruningOptions::nothing(),
        store_precommits: false,
        start_block_height,
    });

    let instance = scraper.build_and_start().await?;

    Ok(instance.storage)
}
