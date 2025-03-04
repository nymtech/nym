use std::{sync::Arc, time::Duration};

use moka::{future::Cache, Entry};
use nym_crypto::asymmetric::ed25519::PublicKey;
use tokio::sync::RwLock;

use crate::{
    db::DbPool,
    http::models::{DailyStats, Gateway, Mixnode, SummaryHistory},
};

use super::models::SessionStats;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    db_pool: DbPool,
    cache: HttpCache,
    agent_key_list: Vec<PublicKey>,
    agent_max_count: i64,
}

impl AppState {
    pub(crate) async fn new(
        db_pool: DbPool,
        cache_ttl: u64,
        agent_key_list: Vec<PublicKey>,
        agent_max_count: i64,
        hm_url: String,
    ) -> Self {
        Self {
            db_pool,
            cache: HttpCache::new(cache_ttl, hm_url).await,
            agent_key_list,
            agent_max_count,
        }
    }

    pub(crate) fn db_pool(&self) -> &DbPool {
        &self.db_pool
    }

    pub(crate) fn cache(&self) -> &HttpCache {
        &self.cache
    }

    pub(crate) fn is_registered(&self, agent_pubkey: &PublicKey) -> bool {
        self.agent_key_list.contains(agent_pubkey)
    }

    pub(crate) fn agent_max_count(&self) -> i64 {
        self.agent_max_count
    }
}

#[derive(Debug, Clone)]
struct HistoricMixingStats {
    historic_stats: Vec<DailyStats>,
}

impl HistoricMixingStats {
    /// Collect historic stats only on initialization. From this point onwards,
    /// service will collect its own stats
    async fn init(hm_url: String) -> Self {
        tracing::info!("Fetching historic mixnode stats from {}", hm_url);

        let target_url = format!("{}/v2/mixnodes/stats", hm_url);
        if let Ok(response) = reqwest::get(&target_url)
            .await
            .and_then(|res| res.error_for_status())
            .inspect_err(|err| tracing::error!("Failed to fetch cache from HM: {}", err))
        {
            if let Ok(mut daily_stats) = response.json::<Vec<DailyStats>>().await {
                // sorting required for seamless comparison later (descending, newest first)
                daily_stats.sort_by(|left, right| right.date_utc.cmp(&left.date_utc));

                tracing::info!(
                    "Successfully fetched {} historic entries from {}",
                    daily_stats.len(),
                    hm_url
                );
                return Self {
                    historic_stats: daily_stats,
                };
            }
        };

        tracing::warn!("Failed to get historic daily stats from {}", hm_url);
        Self {
            historic_stats: Vec::new(),
        }
    }

    /// polyfill with historical data obtained from Harbour Master
    fn merge_with_historic_stats(&self, mut new_stats: Vec<DailyStats>) -> Vec<DailyStats> {
        // newest first
        new_stats.sort_by(|left, right| right.date_utc.cmp(&left.date_utc));

        // historic stats are only used for dates when we don't have new data
        let oldest_date_in_new_stats = new_stats
            .last()
            .map(|day| day.date_utc.to_owned())
            .unwrap_or(String::from("1900-01-01"));

        // given 2 arrays
        // index    historic_stats      new_stats
        //   0        30-01               31-01
        //   1        29-01               30-01
        //   2        28-01
        //            ...
        //   N        01-01
        // cutoff point would be at historic_stats[1]
        // (first date smaller than oldest we've already got)
        if let Some(cutoff) = self
            .historic_stats
            .iter()
            .position(|elem| elem.date_utc < oldest_date_in_new_stats)
        {
            // missing data = (all historic data) - (however many days we already have)
            let missing_data = self.historic_stats.iter().skip(cutoff).cloned();

            // extend new data with missing days
            tracing::debug!(
                "Polyfilled with {} historic records from {:?} to {:?}",
                missing_data.len(),
                self.historic_stats.last(),
                self.historic_stats.get(cutoff)
            );
            new_stats.extend(missing_data);

            // oldest first
            new_stats.into_iter().rev().collect::<Vec<_>>()
        } else {
            // if all historic data is older than what we've got, don't use it
            new_stats
        }
    }
}

static GATEWAYS_LIST_KEY: &str = "gateways";
static MIXNODES_LIST_KEY: &str = "mixnodes";
static MIXSTATS_LIST_KEY: &str = "mixstats";
static SUMMARY_HISTORY_LIST_KEY: &str = "summary-history";
static SESSION_STATS_LIST_KEY: &str = "session-stats";

#[derive(Debug, Clone)]
pub(crate) struct HttpCache {
    gateways: Cache<String, Arc<RwLock<Vec<Gateway>>>>,
    mixnodes: Cache<String, Arc<RwLock<Vec<Mixnode>>>>,
    mixstats: Cache<String, Arc<RwLock<Vec<DailyStats>>>>,
    history: Cache<String, Arc<RwLock<Vec<SummaryHistory>>>>,
    session_stats: Cache<String, Arc<RwLock<Vec<SessionStats>>>>,
    mixnode_historic_daily_stats: HistoricMixingStats,
}

impl HttpCache {
    pub async fn new(ttl_seconds: u64, hm_url: String) -> Self {
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
            session_stats: Cache::builder()
                .max_capacity(2)
                .time_to_live(Duration::from_secs(ttl_seconds))
                .build(),
            mixnode_historic_daily_stats: HistoricMixingStats::init(hm_url).await,
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
                tracing::trace!("Fetching from cache...");
                let read_lock = guard.read().await;
                read_lock.clone()
            }
            None => {
                // the key is missing so populate it
                tracing::trace!("No gateways in cache, refreshing cache from DB...");

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
                tracing::trace!("Fetching from cache...");
                let read_lock = guard.read().await;
                read_lock.clone()
            }
            None => {
                tracing::trace!("No mixnodes in cache, refreshing cache from DB...");

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

    pub async fn get_mixnode_stats(&self, db: &DbPool, offset: i64) -> Vec<DailyStats> {
        match self.mixstats.get(MIXSTATS_LIST_KEY).await {
            Some(guard) => {
                let read_lock = guard.read().await;
                read_lock.to_vec()
            }
            None => {
                let new_node_stats = crate::db::queries::get_daily_stats(db, offset)
                    .await
                    .unwrap_or_default();
                // for every day that's missing, fill it with cached historic data
                let mut mixnode_stats = self
                    .mixnode_historic_daily_stats
                    .merge_with_historic_stats(new_node_stats);
                mixnode_stats.truncate(30);

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

    pub async fn get_sessions_stats(&self, db: &DbPool) -> Vec<SessionStats> {
        match self.session_stats.get(SESSION_STATS_LIST_KEY).await {
            Some(guard) => {
                let read_lock = guard.read().await;
                read_lock.to_vec()
            }
            None => {
                let session_stats = crate::db::queries::get_sessions_stats(db)
                    .await
                    .unwrap_or_default();
                self.upsert_sessions_stats(session_stats.clone()).await;
                session_stats
            }
        }
    }

    pub async fn upsert_sessions_stats(
        &self,
        session_stats: Vec<SessionStats>,
    ) -> Entry<String, Arc<RwLock<Vec<SessionStats>>>> {
        self.session_stats
            .entry_by_ref(SESSION_STATS_LIST_KEY)
            .and_upsert_with(|maybe_entry| async {
                if let Some(entry) = maybe_entry {
                    let v = entry.into_value();
                    let mut guard = v.write().await;
                    *guard = session_stats;
                    v.clone()
                } else {
                    Arc::new(RwLock::new(session_stats))
                }
            })
            .await
    }
}
