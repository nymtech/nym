use crate::config::payments_watcher::HttpAuthenticationOptions;
use crate::config::PaymentWatcherConfig;
use crate::db::queries;
use crate::models::WebhookPayload;
use nym_validator_client::nyxd::AccountId;
use nyxd_scraper::storage::ScraperStorage;
use reqwest::Client;
use rocket::form::validate::Contains;
use serde_json::Value;
use sqlx::SqlitePool;
use std::str::FromStr;
use tokio::time::{self, Duration};
use tracing::{error, info, trace};

#[derive(Debug)]
struct TransferEvent {
    recipient: AccountId,
    sender: AccountId,
    amount: String,
    message_index: u64,
}

pub(crate) async fn run_payment_listener(
    payment_watcher_config: PaymentWatcherConfig,
    watcher_pool: SqlitePool,
    chain_storage: ScraperStorage,
) -> anyhow::Result<()> {
    let client = Client::new();

    let default_message_types = vec!["/cosmos.bank.v1beta1.MsgSend".to_string()];

    loop {
        // 1. get the last height this watcher ran at
        let last_checked_height = queries::payments::get_last_checked_height(&watcher_pool).await?;
        info!("Last checked height: {}", last_checked_height);

        // 2. iterate through watchers
        for watcher in &payment_watcher_config.watchers {
            let watch_for_chain_message_types = watcher
                .watch_for_chain_message_types
                .as_ref()
                .unwrap_or(&default_message_types);

            // 3. build up transactions that match the message types we are looking for
            let mut transactions = vec![];
            for message_type in watch_for_chain_message_types {
                match chain_storage
                    .get_transactions_after_height(
                        last_checked_height,
                        Some(message_type),
                    )
                    .await {
                    Ok(txs) => {
                        for t in txs {
                            transactions.push(t);
                        }
                    }
                    Err(e) => error!("Failed to get transactions (message_type = {message_type}) from scraper database: {e}")
                }
            }

            for tx in transactions {
                if let Some(raw_log) = tx.raw_log.as_deref() {
                    if let Some(watch_for_transfer_recipient_accounts) =
                        &watcher.watch_for_transfer_recipient_accounts
                    {
                        // 4. match recipient accounts we are looking for
                        match parse_transfer_from_raw_log(
                            raw_log,
                            watch_for_transfer_recipient_accounts,
                        ) {
                            Ok(transfer_events) => {
                                if !transfer_events.is_empty() {
                                    info!(
                                    "[watcher = {}] Processing transaction: {} - {} payment events found",
                                    watcher.id, tx.hash, transfer_events.len()
                                );
                                }

                                for transfer in transfer_events {
                                    let amount: f64 = parse_unym_amount(&transfer.amount)?;

                                    queries::payments::insert_payment(
                                        &watcher_pool,
                                        tx.hash.clone(),
                                        transfer.sender.clone().to_string(),
                                        transfer.recipient.clone().to_string(),
                                        amount,
                                        tx.height,
                                        tx.memo.clone(),
                                    )
                                    .await?;

                                    let webhook_data = WebhookPayload {
                                        transaction_hash: tx.hash.clone(),
                                        message_index: transfer.message_index,
                                        sender_address: transfer.sender.to_string(),
                                        receiver_address: transfer.recipient.to_string(),
                                        amount: transfer.amount,
                                        height: tx.height as u128,
                                        memo: tx.memo.clone(),
                                    };

                                    let mut request_builder =
                                        client.post(&watcher.webhook_url).json(&webhook_data);

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
                                            tx.hash,
                                            transfer.message_index,
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
                            Err(e) => error!(
                                "[watcher = {}] ❌ Parse logs for tx {} failed, error = {}",
                                watcher.id, tx.hash, e,
                            ),
                        }
                    }
                }
            }
        }

        time::sleep(Duration::from_secs(10)).await;
    }
}

fn parse_transfer_from_raw_log(
    raw_log: &str,
    watch_for_transfer_recipient_accounts: &Vec<AccountId>,
) -> anyhow::Result<Vec<TransferEvent>> {
    let log_value: Value = serde_json::from_str(raw_log)?;

    let mut transfers: Vec<TransferEvent> = vec![];

    let default_value = vec![];
    let log_entries: &Vec<Value> = log_value.as_array().unwrap_or(&default_value);

    trace!("contains {} log entries", log_entries.len());

    for log_entry in log_entries {
        let message_index = log_entry["msg_index"].as_u64().unwrap_or_default();

        trace!("entry - {message_index}...");

        if let Some(events) = log_entry["events"].as_array() {
            for transfer_event in events.iter().filter(|e| e["type"] == "transfer") {
                if let Some(attrs) = transfer_event["attributes"].as_array() {
                    let mut recipient: Option<AccountId> = None;
                    let mut sender: Option<AccountId> = None;
                    let mut amount: Option<String> = None;

                    for attr in attrs {
                        match attr["key"].as_str() {
                            Some("recipient") => {
                                recipient =
                                    AccountId::from_str(attr["value"].as_str().unwrap_or("")).ok();
                            }
                            Some("sender") => {
                                sender =
                                    AccountId::from_str(attr["value"].as_str().unwrap_or("")).ok();
                            }
                            Some("amount") => {
                                amount = Some(attr["value"].as_str().unwrap_or("").to_string())
                            }
                            // TODO: parse message index
                            _ => continue,
                        }
                    }

                    if let (Some(recipient), Some(sender), Some(amount)) =
                        (recipient, sender, amount)
                    {
                        if watch_for_transfer_recipient_accounts.contains(&recipient) {
                            transfers.push(TransferEvent {
                                recipient,
                                sender,
                                amount,
                                message_index,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(transfers)
}

fn parse_unym_amount(amount: &str) -> anyhow::Result<f64> {
    let amount = amount.trim_end_matches("unym");
    let parsed: f64 = amount.parse()?;
    Ok(parsed / 1_000_000.0)
}
