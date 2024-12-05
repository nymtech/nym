use crate::db::queries;
use nyxd_scraper::storage::ScraperStorage;
use reqwest::Client;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::env;
use tokio::time::{self, Duration};

#[derive(Debug)]
struct TransferEvent {
    recipient: String,
    sender: String,
    amount: String,
}

pub(crate) async fn run_payment_listener(
    watcher_pool: SqlitePool,
    chain_storage: ScraperStorage,
) -> anyhow::Result<()> {
    let payment_receive_address = env::var("PAYMENT_RECEIVE_ADDRESS").map_err(|_| {
        anyhow::anyhow!("Environment variable `PAYMENT_RECEIVE_ADDRESS` not defined")
    })?;
    let webhook_url = env::var("WEBHOOK_URL")
        .map_err(|_| anyhow::anyhow!("Environment variable `WEBHOOK_URL` not defined"))?;

    let client = Client::new();
    loop {
        let last_checked_height =
            queries::payments::get_last_checked_height(&watcher_pool).await?;
        tracing::info!("Last checked height: {}", last_checked_height);

        let transactions = chain_storage
            .get_transactions_after_height(
                last_checked_height,
                Some("/cosmos.bank.v1beta1.MsgSend"),
            )
            .await?;

        for tx in transactions {
            tracing::info!("Processing transaction: {}", tx.hash);
            if let Some(raw_log) = tx.raw_log.as_deref() {
                if let Some(transfer) = parse_transfer_from_raw_log(raw_log)? {
                    if transfer.recipient == payment_receive_address {
                        let amount: f64 = parse_unym_amount(&transfer.amount)?;

                        queries::payments::insert_payment(
                            &watcher_pool,
                            tx.hash.clone(),
                            transfer.sender.clone(),
                            transfer.recipient.clone(),
                            amount,
                            tx.height,
                            tx.memo.clone(),
                        )
                        .await?;

                        let webhook_data = json!({
                            "transaction_hash": tx.hash,
                            "sender_address": transfer.sender,
                            "receiver_address": transfer.recipient,
                            "amount": amount,
                            "height": tx.height,
                            "memo": tx.memo,
                        });
                        let _ = client.post(&webhook_url).json(&webhook_data).send().await;
                    }
                }
            }
        }

        time::sleep(Duration::from_secs(10)).await;
    }
}

fn parse_transfer_from_raw_log(raw_log: &str) -> anyhow::Result<Option<TransferEvent>> {
    let log_value: Value = serde_json::from_str(raw_log)?;

    if let Some(events) = log_value[0]["events"].as_array() {
        if let Some(transfer_event) = events.iter().find(|e| e["type"] == "transfer") {
            if let Some(attrs) = transfer_event["attributes"].as_array() {
                let mut transfer = TransferEvent {
                    recipient: String::new(),
                    sender: String::new(),
                    amount: String::new(),
                };

                for attr in attrs {
                    match attr["key"].as_str() {
                        Some("recipient") => {
                            transfer.recipient = attr["value"].as_str().unwrap_or("").to_string()
                        }
                        Some("sender") => {
                            transfer.sender = attr["value"].as_str().unwrap_or("").to_string()
                        }
                        Some("amount") => {
                            transfer.amount = attr["value"].as_str().unwrap_or("").to_string()
                        }
                        _ => continue,
                    }
                }

                return Ok(Some(transfer));
            }
        }
    }

    Ok(None)
}

fn parse_unym_amount(amount: &str) -> anyhow::Result<f64> {
    let amount = amount.trim_end_matches("unym");
    let parsed: f64 = amount.parse()?;
    Ok(parsed / 1_000_000.0)
}
