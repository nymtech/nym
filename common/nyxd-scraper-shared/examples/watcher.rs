// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nyxd_scraper_shared::error::ScraperError;
use nyxd_scraper_shared::storage::FullBlockInformation;
use nyxd_scraper_shared::watcher::{NyxdWatcher, WatcherConfig};
use nyxd_scraper_shared::{BlockModule, ParsedTransactionResponse, TxModule};

struct FancyBlockModule;

struct FancyTxModule;

#[async_trait::async_trait]
impl BlockModule for FancyBlockModule {
    async fn handle_block(&mut self, block: &FullBlockInformation) -> Result<(), ScraperError> {
        println!("🚀 got new block for height {}", block.block.header.height);

        // should be false
        println!("results scraped: {}", block.results.is_some());
        // should be false
        println!("validators scraped: {}", block.validators.is_some());
        // should be true
        println!("transactions scraped: {}", block.transactions.is_some());

        println!();

        Ok(())
    }
}

#[async_trait::async_trait]
impl TxModule for FancyTxModule {
    async fn handle_tx(&mut self, tx: &ParsedTransactionResponse) -> Result<(), ScraperError> {
        println!(
            "✨ got new tx for height {}: {} ({} msgs)",
            tx.block.header.height,
            tx.hash,
            tx.parsed_messages.len()
        );

        Ok(())
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cfg = WatcherConfig {
        websocket_url: "wss://rpc.nymtech.net/websocket".parse()?,
        rpc_url: "https://rpc.nymtech.net".parse()?,
    };

    let watcher = NyxdWatcher::builder(cfg)
        .with_block_module(FancyBlockModule)
        .with_tx_module(FancyTxModule)
        .build_and_start()
        .await?;

    // run for 30s before shutting down
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

    watcher.stop().await;

    Ok(())
}
