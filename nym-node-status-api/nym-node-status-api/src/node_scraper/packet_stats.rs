use super::helpers::scrape_packet_stats;
use sqlx::SqlitePool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::{debug, error, info, instrument, warn};

use crate::db::models::{InsertStatsRecord, ScraperNodeInfo};
use crate::db::queries;

const PACKET_SCRAPE_INTERVAL: Duration = Duration::from_secs(60 * 60);
const QUEUE_CHECK_INTERVAL: Duration = Duration::from_millis(250);
// TODO dz should be env configurable
const MAX_CONCURRENT_TASKS: usize = 25;

static TASK_COUNTER: AtomicUsize = AtomicUsize::new(0);
static TASK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct PacketScraper {
    pool: SqlitePool,
    packet_queue: Arc<Mutex<Vec<ScraperNodeInfo>>>,
}

impl PacketScraper {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            packet_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn start(&self) {
        self.spawn_packet_scraper().await;
    }

    async fn spawn_packet_scraper(&self) {
        let pool = self.pool.clone();
        let queue = self.packet_queue.clone();
        tracing::info!("Starting packet scraper");

        tokio::spawn(async move {
            loop {
                if let Err(e) = Self::run_packet_scraper(&pool, queue.clone()).await {
                    error!(name: "packet_scraper", "Packet scraper failed: {}", e);
                }
                debug!(name: "packet_scraper", "Sleeping for {}s", PACKET_SCRAPE_INTERVAL.as_secs());
                tokio::time::sleep(PACKET_SCRAPE_INTERVAL).await;
            }
        });
    }

    #[instrument(level = "info", name = "packet_scraper", skip_all)]
    async fn run_packet_scraper(
        pool: &SqlitePool,
        queue: Arc<Mutex<Vec<ScraperNodeInfo>>>,
    ) -> anyhow::Result<()> {
        let nodes = queries::get_nodes_for_scraping(pool).await?;
        {
            // TODO dz why do we use mut queue instead of initializing a new queue for each run?
            let mut queue_lock = queue.lock().await;
            tracing::info!(
                "Adding {} nodes to the queue (queue total={})",
                nodes.len(),
                queue_lock.len()
            );
            queue_lock.extend(nodes);
        }

        let results = Self::process_packet_queue(queue).await;
        queries::batch_store_packet_stats(pool, results)
            .await
            .map_err(|err| anyhow::anyhow!("Failed to store packet stats to DB: {err}"))
    }

    async fn process_packet_queue(
        queue: Arc<Mutex<Vec<ScraperNodeInfo>>>,
    ) -> Arc<Mutex<Vec<InsertStatsRecord>>> {
        let results = Arc::new(Mutex::new(Vec::new()));
        let mut task_set = JoinSet::new();

        loop {
            let running_tasks = TASK_COUNTER.load(Ordering::Relaxed);

            if running_tasks < MAX_CONCURRENT_TASKS {
                let node = {
                    let mut queue_lock = queue.lock().await;
                    if queue_lock.is_empty() {
                        TASK_ID_COUNTER.store(0, Ordering::Relaxed);
                        break;
                    }
                    queue_lock.remove(0)
                };

                TASK_COUNTER.fetch_add(1, Ordering::Relaxed);
                let task_id = TASK_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
                let results_clone = Arc::clone(&results);

                task_set.spawn(async move {
                    match scrape_packet_stats(&node).await {
                        Ok(result) => {
                            // each task contributes their result to a shared vec
                            results_clone.lock().await.push(result);
                            debug!(
                                "📊 ✅ Packet stats task #{} for node {} complete",
                                task_id,
                                node.node_id()
                            )
                        }
                        Err(e) => debug!(
                            "📊 ❌ Packet stats task #{} for {} {} failed: {}",
                            task_id,
                            node.node_kind,
                            node.node_id(),
                            e
                        ),
                    }
                    TASK_COUNTER.fetch_sub(1, Ordering::Relaxed);
                });
            } else {
                tokio::time::sleep(QUEUE_CHECK_INTERVAL).await;
            }
        }

        // wait for all the tasks to complete before returning their results
        let total_count = task_set.len();
        let mut success_count = 0;
        while let Some(res) = task_set.join_next().await {
            if let Err(err) = res {
                warn!("Packet stats task panicked: {err}");
            } else {
                success_count += 1;
            }
        }
        let msg = format!(
            "Successfully completed {}/{} tasks ",
            success_count, total_count
        );
        if success_count != total_count {
            warn!(msg);
        } else {
            info!(msg);
        }

        results
    }
}
