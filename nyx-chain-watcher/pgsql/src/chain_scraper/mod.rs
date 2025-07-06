use crate::config::PaymentWatchersConfig;
use crate::env::vars::{
    NYXD_SCRAPER_START_HEIGHT, NYXD_SCRAPER_UNSAFE_NUKE_DB,
    NYXD_SCRAPER_USE_BEST_EFFORT_START_HEIGHT,
};
use crate::http::state::BankScraperModuleState;
use async_trait::async_trait;
use nym_validator_client::nyxd::{Any, Coin, CosmosCoin, Hash, Msg, MsgSend, Name};
use nyxd_scraper_psql::{
    MsgModule, NyxdScraperTransaction, ParsedTransactionResponse, PostgresNyxdScraper,
    PruningOptions, ScraperError,
};
use sqlx::SqlitePool;
use std::fs;
use tracing::{info, warn};

pub(crate) async fn run_chain_scraper(
    config: &crate::config::Config,
    db_pool: SqlitePool,
    shared_state: BankScraperModuleState,
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
    .with_msg_module(BankScraperModule::new(
        db_pool,
        config.payment_watcher_config.clone(),
        shared_state,
    ));

    let instance = scraper.build_and_start().await?;

    info!("🚧 blocking until the chain has caught up...");
    instance.wait_for_startup_sync().await;

    Ok(instance)
}

pub struct BankScraperModule {
    db_pool: SqlitePool,
    payment_config: PaymentWatchersConfig,
    shared_state: BankScraperModuleState,
}

impl BankScraperModule {
    pub fn new(
        db_pool: SqlitePool,
        payment_config: PaymentWatchersConfig,
        shared_state: BankScraperModuleState,
    ) -> Self {
        Self {
            db_pool,
            payment_config,
            shared_state,
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

    fn get_unym_coin(&self, coins: &[CosmosCoin]) -> Option<Coin> {
        coins
            .iter()
            .find(|coin| coin.denom.as_ref() == "unym")
            .map(|c| c.clone().into())
    }

    // TODO: ideally this should be done by the scraper itself
    fn recover_bank_msg(
        &self,
        tx_hash: Hash,
        index: usize,
        msg: &Any,
    ) -> Result<MsgSend, ScraperError> {
        MsgSend::from_any(msg).map_err(|source| ScraperError::MsgParseFailure {
            hash: tx_hash,
            index,
            type_url: self.type_url(),
            source,
        })
    }
}
#[async_trait]
impl MsgModule for BankScraperModule {
    fn type_url(&self) -> String {
        <MsgSend as Msg>::Proto::type_url()
    }

    async fn handle_msg(
        &mut self,
        index: usize,
        msg: &Any,
        tx: &ParsedTransactionResponse,
        _storage_tx: &mut dyn NyxdScraperTransaction,
    ) -> Result<(), ScraperError> {
        let memo = tx.tx.body.memo.clone();

        // Don't process failed transactions
        if !tx.tx_result.code.is_ok() {
            return Ok(());
        }

        let msg = self.recover_bank_msg(tx.hash, index, msg)?;

        // Check if any watcher is watching this recipient
        let is_watched = self
            .payment_config
            .is_being_watched(msg.to_address.as_ref());

        self.shared_state
            .new_bank_msg(tx, index, &msg, is_watched)
            .await;

        if is_watched {
            let Some(unym_coin) = self.get_unym_coin(&msg.amount) else {
                let warn = format!(
                    "{} sent {:?} instead of unym!",
                    msg.from_address, msg.amount
                );
                warn!("{warn}");
                self.shared_state
                    .new_rejection(tx.hash.to_string(), tx.height.value(), index as u32, warn)
                    .await;

                // we don't want to fail the whole processing - this is not a failure in that sense!
                return Ok(());
            };

            if let Err(err) = self
                .store_transfer_event(
                    &tx.hash.to_string(),
                    tx.height.value() as i64,
                    index as i64,
                    msg.from_address.to_string(),
                    msg.to_address.to_string(),
                    unym_coin.to_string(),
                    Some(memo.clone()),
                )
                .await
            {
                warn!("Failed to store transfer event: {err}");
                self.shared_state
                    .new_rejection(
                        tx.hash.to_string(),
                        tx.height.value(),
                        index as u32,
                        format!("storage failure: {err}"),
                    )
                    .await;
            }
        }

        Ok(())
    }
}
