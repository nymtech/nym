use crate::db::models::CoingeckoPriceResponse;
use crate::db::DbPool;
use crate::http::models::status::Webhook;
use axum::extract::FromRef;
use nym_bin_common::bin_info;
use nym_bin_common::build_information::BinaryBuildInformation;
use std::ops::Deref;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tokio::time::Instant;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    db_pool: DbPool,
    #[allow(dead_code)]
    pub(crate) registered_webhooks: Arc<Vec<Webhook>>,
    pub(crate) status_state: StatusState,
    pub(crate) price_scraper_state: PriceScraperState,
}

impl AppState {
    pub(crate) fn new(
        db_pool: DbPool,
        registered_payment_watchers: Vec<Webhook>,
        price_scraper_state: PriceScraperState,
    ) -> Self {
        Self {
            db_pool,
            registered_webhooks: Arc::new(registered_payment_watchers),
            status_state: Default::default(),
            price_scraper_state,
        }
    }

    pub(crate) fn db_pool(&self) -> &DbPool {
        &self.db_pool
    }
}

#[derive(Clone, Debug)]
pub(crate) struct StatusState {
    inner: Arc<StatusStateInner>,
}

impl Default for StatusState {
    fn default() -> Self {
        StatusState {
            inner: Arc::new(StatusStateInner {
                startup_time: Instant::now(),
                build_information: bin_info!(),
            }),
        }
    }
}

impl Deref for StatusState {
    type Target = StatusStateInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
pub(crate) struct StatusStateInner {
    pub(crate) startup_time: Instant,
    pub(crate) build_information: BinaryBuildInformation,
}

#[derive(Debug, Clone)]
pub(crate) struct PriceScraperState {
    pub(crate) inner: Arc<RwLock<PriceScraperStateInner>>,
}

impl PriceScraperState {
    pub(crate) fn new() -> Self {
        PriceScraperState {
            inner: Arc::new(Default::default()),
        }
    }

    pub(crate) async fn new_failure<S: Into<String>>(&self, error: S) {
        self.inner.write().await.last_failure = Some(PriceScraperLastError {
            timestamp: OffsetDateTime::now_utc(),
            message: error.into(),
        })
    }
    pub(crate) async fn new_success(&self, response: CoingeckoPriceResponse) {
        self.inner.write().await.last_success = Some(PriceScraperLastSuccess {
            timestamp: OffsetDateTime::now_utc(),
            response,
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct PriceScraperStateInner {
    pub(crate) last_success: Option<PriceScraperLastSuccess>,
    pub(crate) last_failure: Option<PriceScraperLastError>,
}

#[derive(Debug)]
pub(crate) struct PriceScraperLastSuccess {
    pub(crate) timestamp: OffsetDateTime,
    pub(crate) response: CoingeckoPriceResponse,
}

#[derive(Debug)]
pub(crate) struct PriceScraperLastError {
    pub(crate) timestamp: OffsetDateTime,
    pub(crate) message: String,
}

impl FromRef<AppState> for StatusState {
    fn from_ref(input: &AppState) -> Self {
        input.status_state.clone()
    }
}

impl FromRef<AppState> for PriceScraperState {
    fn from_ref(input: &AppState) -> Self {
        input.price_scraper_state.clone()
    }
}
