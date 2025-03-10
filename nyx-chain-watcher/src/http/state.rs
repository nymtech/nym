use crate::db::DbPool;
use crate::helpers::RingBuffer;
use crate::http::models::status::PaymentWatcher;
use crate::models::WebhookPayload;
use axum::extract::FromRef;
use nym_validator_client::nyxd::Coin;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    db_pool: DbPool,
    pub(crate) registered_payment_watchers: Arc<Vec<PaymentWatcher>>,
    pub(crate) payment_listener_state: PaymentListenerState,
}

impl AppState {
    pub(crate) fn new(
        db_pool: DbPool,
        registered_payment_watchers: Vec<PaymentWatcher>,
        payment_listener_state: PaymentListenerState,
    ) -> Self {
        Self {
            db_pool,
            registered_payment_watchers: Arc::new(registered_payment_watchers),
            payment_listener_state,
        }
    }

    pub(crate) fn db_pool(&self) -> &DbPool {
        &self.db_pool
    }

    pub(crate) fn watched_accounts(&self) -> Vec<String> {
        self.registered_payment_watchers
            .iter()
            .flat_map(|w| w.watched_accounts.iter())
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PaymentListenerState {
    pub(crate) inner: Arc<RwLock<PaymentListenerStateInner>>,
}

impl PaymentListenerState {
    // TODO: make those configurable
    const MAX_WATCHER_FAILURES: usize = 20;
    const MAX_PAYMENT_LISTENER_FAILURES: usize = 50;

    pub(crate) fn new() -> Self {
        PaymentListenerState {
            inner: Arc::new(RwLock::new(PaymentListenerStateInner {
                last_checked: OffsetDateTime::UNIX_EPOCH,
                processed_payments_since_startup: 0,
                watcher_errors_since_startup: 0,
                payment_listener_errors_since_startup: 0,
                last_processed_payment: None,
                latest_failures: RingBuffer::new(Self::MAX_PAYMENT_LISTENER_FAILURES),
                watchers: Default::default(),
            })),
        }
    }

    pub(crate) async fn insert_listener_failure(&self, failure: PaymentListenerFailureDetails) {
        let mut guard = self.inner.write().await;

        guard.payment_listener_errors_since_startup += 1;
        guard.latest_failures.push(failure);
    }

    pub(crate) async fn insert_watcher_failure(&self, id: &str, failure: WatcherFailureDetails) {
        self.inner
            .write()
            .await
            .watchers
            .entry(id.to_string())
            .or_insert(WatcherState {
                latest_failures: RingBuffer::new(Self::MAX_WATCHER_FAILURES),
            })
            .latest_failures
            .push(failure);
    }

    pub(crate) async fn processed_payment_transaction(&self, payment: ProcessedPayment) {
        let mut guard = self.inner.write().await;

        guard.processed_payments_since_startup += 1;
        guard.last_processed_payment = Some(payment)
    }

    pub(crate) async fn update_last_checked(&self) {
        self.inner.write().await.last_checked = OffsetDateTime::now_utc();
    }
}

impl FromRef<AppState> for PaymentListenerState {
    fn from_ref(input: &AppState) -> Self {
        input.payment_listener_state.clone()
    }
}

#[derive(Debug)]
pub(crate) struct PaymentListenerStateInner {
    pub(crate) last_checked: OffsetDateTime,

    pub(crate) processed_payments_since_startup: u64,
    pub(crate) watcher_errors_since_startup: u64,
    pub(crate) payment_listener_errors_since_startup: u64,

    pub(crate) last_processed_payment: Option<ProcessedPayment>,

    pub(crate) latest_failures: RingBuffer<PaymentListenerFailureDetails>,
    pub(crate) watchers: HashMap<String, WatcherState>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ProcessedPayment {
    pub processed_at: OffsetDateTime,

    pub tx_hash: String,
    pub message_index: u64,
    pub height: u64,
    pub sender: String,
    pub receiver: String,
    pub funds: Coin,
    pub memo: String,
}

impl From<WebhookPayload> for ProcessedPayment {
    fn from(payload: WebhookPayload) -> Self {
        ProcessedPayment {
            processed_at: OffsetDateTime::now_utc(),
            tx_hash: payload.transaction_hash,
            message_index: payload.message_index,
            height: payload.height as u64,
            sender: payload.sender_address,
            receiver: payload.receiver_address,
            funds: payload.funds.into(),
            memo: payload.memo.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PaymentListenerFailureDetails {
    pub(crate) timestamp: OffsetDateTime,
    pub(crate) error: String,
}

impl PaymentListenerFailureDetails {
    pub(crate) fn new(error: String) -> Self {
        PaymentListenerFailureDetails {
            timestamp: OffsetDateTime::now_utc(),
            error,
        }
    }
}

#[derive(Debug)]
pub(crate) struct WatcherState {
    pub(crate) latest_failures: RingBuffer<WatcherFailureDetails>,
}

#[derive(Debug)]
pub(crate) struct WatcherFailureDetails {
    pub(crate) timestamp: OffsetDateTime,
    pub(crate) error: String,
}

impl WatcherFailureDetails {
    pub(crate) fn new(error: String) -> Self {
        WatcherFailureDetails {
            timestamp: OffsetDateTime::now_utc(),
            error,
        }
    }
}
