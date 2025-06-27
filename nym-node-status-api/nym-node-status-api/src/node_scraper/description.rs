use super::helpers::scrape_and_store_description;
use anyhow::Result;
use sqlx::SqlitePool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, error, instrument, warn};

use crate::db::models::ScraperNodeInfo;
use crate::db::queries::get_nodes_for_scraping;

const DESCRIPTION_SCRAPE_INTERVAL: Duration = Duration::from_secs(60 * 60 * 4);
const QUEUE_CHECK_INTERVAL: Duration = Duration::from_millis(250);
const MAX_CONCURRENT_TASKS: usize = 5;

static TASK_COUNTER: AtomicUsize = AtomicUsize::new(0);
static TASK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct DescriptionScraper {
    pool: SqlitePool,
    description_queue: Arc<Mutex<Vec<ScraperNodeInfo>>>,
}

impl DescriptionScraper {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            description_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn start(&self) {
        self.spawn_description_scraper().await;
    }

    async fn spawn_description_scraper(&self) {
        let pool = self.pool.clone();
        let queue = self.description_queue.clone();
        tracing::info!("Starting description scraper");
        tokio::spawn(async move {
            loop {
                if let Err(e) = Self::run_description_scraper(&pool, queue.clone()).await {
                    error!(name: "description_scraper", "Description scraper failed: {}", e);
                }
                debug!(name: "description_scraper", "Sleeping for {}s", DESCRIPTION_SCRAPE_INTERVAL.as_secs());
                tokio::time::sleep(DESCRIPTION_SCRAPE_INTERVAL).await;
            }
        });
    }

    #[instrument(level = "info", name = "description_scraper", skip_all)]
    async fn run_description_scraper(
        pool: &SqlitePool,
        queue: Arc<Mutex<Vec<ScraperNodeInfo>>>,
    ) -> Result<()> {
        let nodes = get_nodes_for_scraping(pool).await?;
        if let Ok(mut queue_lock) = queue.lock() {
            queue_lock.extend(nodes);
        } else {
            warn!("Failed to acquire description queue lock");
            return Ok(());
        }

        Self::process_description_queue(pool, queue).await;
        Ok(())
    }

    async fn process_description_queue(pool: &SqlitePool, queue: Arc<Mutex<Vec<ScraperNodeInfo>>>) {
        loop {
            let running_tasks = TASK_COUNTER.load(Ordering::Relaxed);

            if running_tasks < MAX_CONCURRENT_TASKS {
                let node = {
                    if let Ok(mut queue_lock) = queue.lock() {
                        if queue_lock.is_empty() {
                            TASK_ID_COUNTER.store(0, Ordering::Relaxed);
                            break;
                        }
                        queue_lock.remove(0)
                    } else {
                        warn!("Failed to acquire description queue lock");
                        break;
                    }
                };

                TASK_COUNTER.fetch_add(1, Ordering::Relaxed);
                let task_id = TASK_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
                let pool = pool.clone();

                tokio::spawn(async move {
                    match scrape_and_store_description(&pool, &node).await {
                        Ok(_) => debug!(
                            "📝 ✅ Description task #{} for node {} complete",
                            task_id,
                            node.node_id()
                        ),
                        Err(e) => debug!(
                            "📝 ❌ Description task #{} for {} {} failed: {}",
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

        // TODO After all tasks complete, write results to the DB
    }
}
