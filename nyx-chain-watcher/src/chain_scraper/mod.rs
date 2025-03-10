use crate::config::PaymentWatchersConfig;
use crate::env::vars::{
    NYXD_SCRAPER_START_HEIGHT, NYXD_SCRAPER_UNSAFE_NUKE_DB,
    NYXD_SCRAPER_USE_BEST_EFFORT_START_HEIGHT,
};
use async_trait::async_trait;
use nyxd_scraper::{
    error::ScraperError, storage::StorageTransaction, NyxdScraper, ParsedTransactionResponse,
    PruningOptions, TxModule,
};
use sqlx::SqlitePool;
use std::fs;
use tracing::{error, info, warn};

pub(crate) async fn run_chain_scraper(
    config: &crate::config::Config,
    db_pool: SqlitePool,
) -> anyhow::Result<NyxdScraper> {
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
        fs::remove_file(config.chain_scraper_database_path())?;
    }

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
    })
    .with_tx_module(EventScraperModule::new(
        db_pool,
        config.payment_watcher_config.clone(),
    ));

    let instance = scraper.build_and_start().await?;

    info!("🚧 blocking until the chain has caught up...");
    instance.wait_for_startup_sync().await;

    Ok(instance)
}

pub struct EventScraperModule {
    db_pool: SqlitePool,
    payment_config: PaymentWatchersConfig,
}

impl EventScraperModule {
    pub fn new(db_pool: SqlitePool, payment_config: PaymentWatchersConfig) -> Self {
        Self {
            db_pool,
            payment_config,
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn store_transfer_event(
        &self,
        tx_hash: &str,
        height: i64,
        message_index: i64,
        sender: String,
        recipient: String,
        amount: String,
        memo: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO transactions (tx_hash, height, message_index, sender, recipient, amount, memo)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            tx_hash,
            height,
            message_index,
            sender,
            recipient,
            amount,
            memo
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl TxModule for EventScraperModule {
    async fn handle_tx(
        &mut self,
        tx: &ParsedTransactionResponse,
        _: &mut StorageTransaction,
    ) -> Result<(), ScraperError> {
        let events = &tx.tx_result.events;
        let height = tx.height.value() as i64;
        let tx_hash = tx.hash.to_string();
        let memo = tx.tx.body.memo.clone();

        // Don't process failed transactions
        if !tx.tx_result.code.is_ok() {
            return Ok(());
        }

        if tx.tx.body.messages.len() > 1 {
            error!(
                "this transaction has more than 1 message in it - payment information will be lost"
            );
        }

        // Process each event
        for event in events {
            // Only process transfer events
            if event.kind == "transfer" {
                let mut recipient = None;
                let mut sender = None;
                let mut amount = None;
                // TODO: get message index from event
                let message_index = 0;

                // Extract transfer event attributes
                for attr in &event.attributes {
                    if let (Ok(key), Ok(value)) = (attr.key_str(), attr.value_str()) {
                        match key {
                            "recipient" => recipient = Some(value.to_string()),
                            "sender" => sender = Some(value.to_string()),
                            "amount" => amount = Some(value.to_string()),
                            _ => continue,
                        }
                    }
                }

                // If we have all required fields, check if recipient is watched and store
                if let (Some(recipient), Some(sender), Some(amount)) = (recipient, sender, amount) {
                    // Check if any watcher is watching this recipient
                    let is_watched = self.payment_config.is_being_watched(&recipient);

                    if is_watched {
                        if let Err(e) = self
                            .store_transfer_event(
                                &tx_hash,
                                height,
                                message_index,
                                sender,
                                recipient,
                                amount,
                                Some(memo.clone()),
                            )
                            .await
                        {
                            warn!("Failed to store transfer event: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
