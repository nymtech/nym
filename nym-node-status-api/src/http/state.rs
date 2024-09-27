use std::{sync::Arc, time::Duration};

use moka::{future::Cache, Entry};
use tokio::sync::RwLock;

use crate::{
    db::DbPool,
    http::models::{DailyStats, Gateway, Mixnode, SummaryHistory},
};

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    db_pool: DbPool,
    cache: HttpCache,
}

impl AppState {
    pub(crate) fn new(db_pool: DbPool, cache_ttl: u64) -> Self {
        Self {
            db_pool,
            cache: HttpCache::new(cache_ttl),
        }
    }

    pub(crate) fn db_pool(&self) -> &DbPool {
        &self.db_pool
    }

    pub(crate) fn cache(&self) -> &HttpCache {
        &self.cache
    }
}

static GATEWAYS_LIST_KEY: &str = "gateways";
static MIXNODES_LIST_KEY: &str = "mixnodes";
static MIXSTATS_LIST_KEY: &str = "mixstats";
static SUMMARY_HISTORY_LIST_KEY: &str = "summary-history";

#[derive(Debug, Clone)]
pub(crate) struct HttpCache {
    gateways: Cache<String, Arc<RwLock<Vec<Gateway>>>>,
    mixnodes: Cache<String, Arc<RwLock<Vec<Mixnode>>>>,
    mixstats: Cache<String, Arc<RwLock<Vec<DailyStats>>>>,
    history: Cache<String, Arc<RwLock<Vec<SummaryHistory>>>>,
}

impl HttpCache {
    pub fn new(ttl_seconds: u64) -> Self {
        HttpCache {
            gateways: Cache::builder()
                .max_capacity(2)
                .time_to_live(Duration::from_secs(ttl_seconds))
                .build(),
            mixnodes: Cache::builder()
                .max_capacity(2)
                .time_to_live(Duration::from_secs(ttl_seconds))
                .build(),
            mixstats: Cache::builder()
                .max_capacity(2)
                .time_to_live(Duration::from_secs(ttl_seconds))
                .build(),
            history: Cache::builder()
                .max_capacity(2)
                .time_to_live(Duration::from_secs(ttl_seconds))
                .build(),
        }
    }

    pub async fn upsert_gateway_list(
        &self,
        new_gateway_list: Vec<Gateway>,
    ) -> Entry<String, Arc<RwLock<Vec<Gateway>>>> {
        self.gateways
            .entry_by_ref(GATEWAYS_LIST_KEY)
            .and_upsert_with(|maybe_entry| async {
                if let Some(entry) = maybe_entry {
                    let v = entry.into_value();
                    let mut guard = v.write().await;
                    *guard = new_gateway_list;
                    v.clone()
                } else {
                    Arc::new(RwLock::new(new_gateway_list))
                }
            })
            .await
    }

    pub async fn get_gateway_list(&self, db: &DbPool) -> Vec<Gateway> {
        match self.gateways.get(GATEWAYS_LIST_KEY).await {
            Some(guard) => {
                let read_lock = guard.read().await;
                read_lock.clone()
            }
            None => {
                // the key is missing so populate it
                tracing::warn!("No gateways in cache, refreshing cache from DB...");

                let gateways = crate::db::queries::get_all_gateways(db)
                    .await
                    .unwrap_or_default();
                self.upsert_gateway_list(gateways.clone()).await;

                if gateways.is_empty() {
                    tracing::warn!("Database contains 0 gateways");
                }

                gateways
            }
        }
    }

    pub async fn upsert_mixnode_list(
        &self,
        new_mixnode_list: Vec<Mixnode>,
    ) -> Entry<String, Arc<RwLock<Vec<Mixnode>>>> {
        self.mixnodes
            .entry_by_ref(MIXNODES_LIST_KEY)
            .and_upsert_with(|maybe_entry| async {
                if let Some(entry) = maybe_entry {
                    let v = entry.into_value();
                    let mut guard = v.write().await;
                    *guard = new_mixnode_list;
                    v.clone()
                } else {
                    Arc::new(RwLock::new(new_mixnode_list))
                }
            })
            .await
    }

    pub async fn get_mixnodes_list(&self, db: &DbPool) -> Vec<Mixnode> {
        match self.mixnodes.get(MIXNODES_LIST_KEY).await {
            Some(guard) => {
                let read_lock = guard.read().await;
                read_lock.clone()
            }
            None => {
                tracing::warn!("No mixnodes in cache, refreshing cache from DB...");

                let mixnodes = crate::db::queries::get_all_mixnodes(db)
                    .await
                    .unwrap_or_default();
                self.upsert_mixnode_list(mixnodes.clone()).await;

                if mixnodes.is_empty() {
                    tracing::warn!("Database contains 0 mixnodes");
                }

                mixnodes
            }
        }
    }

    pub async fn upsert_mixnode_stats(
        &self,
        mixnode_stats: Vec<DailyStats>,
    ) -> Entry<String, Arc<RwLock<Vec<DailyStats>>>> {
        self.mixstats
            .entry_by_ref(MIXSTATS_LIST_KEY)
            .and_upsert_with(|maybe_entry| async {
                if let Some(entry) = maybe_entry {
                    let v = entry.into_value();
                    let mut guard = v.write().await;
                    *guard = mixnode_stats;
                    v.clone()
                } else {
                    Arc::new(RwLock::new(mixnode_stats))
                }
            })
            .await
    }

    pub async fn get_mixnode_stats(&self, db: &DbPool) -> Vec<DailyStats> {
        match self.mixstats.get(MIXSTATS_LIST_KEY).await {
            Some(guard) => {
                let read_lock = guard.read().await;
                read_lock.to_vec()
            }
            None => {
                let mixnode_stats = crate::db::queries::get_daily_stats(db)
                    .await
                    .unwrap_or_default();
                self.upsert_mixnode_stats(mixnode_stats.clone()).await;
                mixnode_stats
            }
        }
    }

    pub async fn get_summary_history(&self, db: &DbPool) -> Vec<SummaryHistory> {
        match self.history.get(SUMMARY_HISTORY_LIST_KEY).await {
            Some(guard) => {
                let read_lock = guard.read().await;
                read_lock.to_vec()
            }
            None => {
                let summary_history = crate::db::queries::get_summary_history(db)
                    .await
                    .unwrap_or(vec![]);
                self.upsert_summary_history(summary_history.clone()).await;
                summary_history
            }
        }
    }

    pub async fn upsert_summary_history(
        &self,
        summary_history: Vec<SummaryHistory>,
    ) -> Entry<String, Arc<RwLock<Vec<SummaryHistory>>>> {
        self.history
            .entry_by_ref(SUMMARY_HISTORY_LIST_KEY)
            .and_upsert_with(|maybe_entry| async {
                if let Some(entry) = maybe_entry {
                    let v = entry.into_value();
                    let mut guard = v.write().await;
                    *guard = summary_history;
                    v.clone()
                } else {
                    Arc::new(RwLock::new(summary_history))
                }
            })
            .await
    }
}
