use crate::config::payments_watcher::HttpAuthenticationOptions;
use crate::config::PaymentWatcherConfig;
use crate::db::queries;
use crate::models::WebhookPayload;
use nym_validator_client::nyxd::Coin;
use reqwest::Client;
use sqlx::SqlitePool;
use std::str::FromStr;
use tokio::time::{self, Duration};
use tracing::{error, info};

pub(crate) async fn run_payment_listener(
    payment_watcher_config: PaymentWatcherConfig,
    watcher_pool: SqlitePool,
) -> anyhow::Result<()> {
    let client = Client::new();

    loop {
        // 1. get the last height this watcher ran at
        let last_checked_height = queries::payments::get_last_checked_height(&watcher_pool).await?;
        info!("Last checked height: {}", last_checked_height);

        // 2. iterate through watchers
        for watcher in &payment_watcher_config.watchers {
            if watcher.watch_for_transfer_recipient_accounts.is_some() {
                // 3. Query new transactions for this watcher's recipient accounts
                let transactions = sqlx::query!(
                    r#"
                    SELECT * FROM transactions
                    WHERE height > ? 
                    ORDER BY height ASC, message_index ASC
                    "#,
                    last_checked_height
                )
                .fetch_all(&watcher_pool)
                .await?;

                if !transactions.is_empty() {
                    info!(
                        "[watcher = {}] Processing {} transactions",
                        watcher.id,
                        transactions.len()
                    );
                }

                for tx in transactions {
                    let funds = Coin::from_str(&tx.amount)?;
                    let amount: f64 = funds.amount as f64 / 1e6f64; // convert to major value, there will be precision loss

                    // Store transaction hash for later use
                    let tx_hash = tx.tx_hash.clone();
                    let message_index = tx.message_index;

                    queries::payments::insert_payment(
                        &watcher_pool,
                        tx.tx_hash,
                        tx.sender.clone(),
                        tx.recipient.clone(),
                        amount,
                        tx.height,
                        tx.memo.clone(),
                    )
                    .await?;

                    let webhook_data = WebhookPayload {
                        transaction_hash: tx_hash.clone(),
                        message_index: message_index as u64,
                        sender_address: tx.sender,
                        receiver_address: tx.recipient,
                        funds: funds.into(),
                        height: tx.height as u128,
                        memo: tx.memo,
                    };

                    let mut request_builder = client.post(&watcher.webhook_url).json(&webhook_data);

                    if let Some(auth) = &watcher.authentication {
                        match auth {
                            HttpAuthenticationOptions::AuthorizationBearerToken { token } => {
                                request_builder = request_builder.bearer_auth(token);
                            }
                        }
                    }

                    match request_builder.send().await {
                        Ok(res) => info!(
                            "[watcher = {}] ✅ Webhook {} {} - tx {}, index {}",
                            watcher.id,
                            res.status(),
                            res.url(),
                            tx_hash,
                            message_index,
                        ),
                        Err(e) => error!(
                            "[watcher = {}] ❌ Webhook {:?} {:?} error = {}",
                            watcher.id,
                            e.status(),
                            e.url(),
                            e,
                        ),
                    }
                }
            }
        }

        time::sleep(Duration::from_secs(10)).await;
    }
}
