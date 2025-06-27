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

static TASK_COUNTER: AtomicUsize = AtomicUsize::new(0);
static TASK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct PacketScraper {
    pool: SqlitePool,
    max_concurrent_tasks: usize,
}

impl PacketScraper {
    pub fn new(pool: SqlitePool, max_concurrent_tasks: usize) -> Self {
        Self {
            pool,
            max_concurrent_tasks,
        }
    }

    pub async fn start(&self) {
        self.spawn_packet_scraper().await;
    }

    async fn spawn_packet_scraper(&self) {
        let pool = self.pool.clone();
        tracing::info!("Starting packet scraper");
        let max_concurrent_tasks = self.max_concurrent_tasks;

        tokio::spawn(async move {
            loop {
                if let Err(e) = Self::run_packet_scraper(&pool, max_concurrent_tasks).await {
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
        max_concurrent_tasks: usize,
    ) -> anyhow::Result<()> {
        let queue = queries::get_nodes_for_scraping(pool).await?;
        tracing::info!("Adding {} nodes to the queue", queue.len(),);

        let results = Self::process_packet_queue(queue, max_concurrent_tasks).await;
        queries::batch_store_packet_stats(pool, results)
            .await
            .map_err(|err| anyhow::anyhow!("Failed to store packet stats to DB: {err}"))
    }

    async fn process_packet_queue(
        queue: Vec<ScraperNodeInfo>,
        max_concurrent_tasks: usize,
    ) -> Arc<Mutex<Vec<InsertStatsRecord>>> {
        let mut queue = queue;
        let results = Arc::new(Mutex::new(Vec::new()));
        let mut task_set = JoinSet::new();

        loop {
            let running_tasks = TASK_COUNTER.load(Ordering::Relaxed);

            if running_tasks < max_concurrent_tasks {
                let node = {
                    if queue.is_empty() {
                        TASK_ID_COUNTER.store(0, Ordering::Relaxed);
                        break;
                    }
                    queue.remove(0)
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
