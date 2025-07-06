// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::PaymentWatchersConfig;
use crate::db::models::Transaction;
use crate::db::{queries, DbPool};
use crate::http::state::{
    PaymentListenerFailureDetails, PaymentListenerState, ProcessedPayment, WatcherFailureDetails,
};
use crate::models::WebhookPayload;
use crate::payment_listener::watcher::PaymentWatcher;
use anyhow::Context;
use tokio::time::{self, Duration};
use tracing::{debug, error, info};

pub(crate) mod watcher;

pub(crate) struct PaymentListener {
    db_pool: DbPool,
    payment_watchers: Vec<PaymentWatcher>,
    shared_state: PaymentListenerState,
}

impl PaymentListener {
    pub(crate) fn new(
        db_pool: DbPool,
        config: PaymentWatchersConfig,
        shared_state: PaymentListenerState,
    ) -> anyhow::Result<Self> {
        Ok(PaymentListener {
            db_pool,
            payment_watchers: config
                .watchers
                .iter()
                .map(|watcher_cfg| PaymentWatcher::new(watcher_cfg.clone()))
                .collect::<anyhow::Result<Vec<_>>>()?,
            shared_state,
        })
    }

    async fn retrieve_unprocessed_transactions(&self) -> anyhow::Result<Vec<Transaction>> {
        let last_checked_height = queries::payments::get_last_checked_height(&self.db_pool).await?;
        let txs = sqlx::query_as!(
            Transaction,
            r#"
                SELECT id, tx_hash, height, message_index, sender, recipient, amount, memo, created_at as "created_at: ::time::OffsetDateTime"
                FROM transactions
                WHERE height > $1
                ORDER BY height, message_index
            "#,
            last_checked_height
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(txs)
    }

    async fn process_transaction(&self, tx: Transaction) -> anyhow::Result<()> {
        // 3.1 process any payments
        let funds = tx.funds()?;
        let amount: f64 = funds.amount as f64 / 1e6f64; // convert to major value, there will be precision loss

        // TODO: FIXME: it may happen that we insert a payment but fail to invoke all webhooks

        queries::payments::insert_payment(
            &self.db_pool,
            tx.tx_hash.clone(),
            tx.sender.clone(),
            tx.recipient.clone(),
            amount,
            tx.height,
            tx.memo.clone(),
        )
        .await?;

        // 3.1. invoke all relevant webhooks for all registered watchers
        let webhook_data = WebhookPayload {
            transaction_hash: tx.tx_hash,
            message_index: tx.message_index as u64,
            sender_address: tx.sender,
            receiver_address: tx.recipient,
            funds: funds.into(),
            height: tx.height as u128,
            memo: tx.memo,
        };

        for watcher in &self.payment_watchers {
            if let Err(err) = watcher.invoke_webhook(&webhook_data).await {
                error!("watcher {} failure: {err:#}", watcher.id());
                self.shared_state
                    .insert_watcher_failure(
                        watcher.id(),
                        WatcherFailureDetails::new(err.to_string()),
                    )
                    .await
            }
        }

        self.shared_state
            .processed_payment_transaction(ProcessedPayment::from(webhook_data))
            .await;

        Ok(())
    }

    async fn check_for_unprocessed_payments(&self) -> anyhow::Result<()> {
        // 1. retrieve any unprocessed transactions
        let unprocessed_transactions = self
            .retrieve_unprocessed_transactions()
            .await
            .context("failed to retrieve unprocessed transactions")?;

        if unprocessed_transactions.is_empty() {
            debug!("no payment transactions to process.");
            return Ok(());
        } else {
            info!(
                "processing {} payment transactions",
                unprocessed_transactions.len()
            );
        }

        // 2. attempt to process them
        for tx in unprocessed_transactions {
            let hash = tx.tx_hash.clone();
            let height = tx.height;
            self.process_transaction(tx).await.with_context(|| {
                format!("failed to process transaction {hash} at height {height}")
            })?;
        }

        Ok(())
    }

    pub(crate) async fn run(&self) {
        loop {
            time::sleep(Duration::from_secs(10)).await;

            if let Err(err) = self.check_for_unprocessed_payments().await {
                error!("failed to fully process payments: {err:#}");
                self.shared_state
                    .insert_listener_failure(PaymentListenerFailureDetails::new(err.to_string()))
                    .await;
                continue;
            }

            self.shared_state.update_last_checked().await;
        }
    }
}
