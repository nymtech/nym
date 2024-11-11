use nyxd_scraper::{storage::ScraperStorage, Config, NyxdScraper, PruningOptions};

pub(crate) async fn run_chain_scraper() -> anyhow::Result<ScraperStorage> {
    let websocket_url =
        std::env::var("NYXD_WEBSOCKET_URL").expect("NYXD_WEBSOCKET_URL not defined");

    let rpc_url = std::env::var("NYXD_RPC_URL").expect("NYXD_RPC_URL not defined");
    let websocket_url = reqwest::Url::parse(&websocket_url)?;
    let rpc_url = reqwest::Url::parse(&rpc_url)?;

    let scraper = NyxdScraper::builder(Config {
        websocket_url,
        rpc_url,
        database_path: "chain_history.sqlite".into(),
        pruning_options: PruningOptions::nothing(),
        store_precommits: false,
    });

    let storage = scraper.build_and_start().await?;

    Ok(storage.storage)
}
