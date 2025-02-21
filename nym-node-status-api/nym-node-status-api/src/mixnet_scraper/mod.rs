use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
pub mod helpers;
use anyhow::Result;
use helpers::{scrape_and_store_description, scrape_and_store_packet_stats};
use sqlx::SqlitePool;
use tracing::{debug, error, instrument, warn};

use crate::db::models::ScraperNodeInfo;
use crate::db::queries::get_nodes_for_scraping;

const DESCRIPTION_SCRAPE_INTERVAL: Duration = Duration::from_secs(60 * 60 * 4);
const PACKET_SCRAPE_INTERVAL: Duration = Duration::from_secs(60 * 60);
const QUEUE_CHECK_INTERVAL: Duration = Duration::from_millis(250);
const MAX_CONCURRENT_TASKS: usize = 5;

static TASK_COUNTER: AtomicUsize = AtomicUsize::new(0);
static TASK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct Scraper {
    pool: SqlitePool,
    description_queue: Arc<Mutex<Vec<ScraperNodeInfo>>>,
    packet_queue: Arc<Mutex<Vec<ScraperNodeInfo>>>,
}

impl Scraper {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            description_queue: Arc::new(Mutex::new(Vec::new())),
            packet_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn start(&self) {
        self.spawn_description_scraper().await;
        self.spawn_packet_scraper().await;
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

    #[instrument(level = "info", name = "packet_scraper", skip_all)]
    async fn run_packet_scraper(
        pool: &SqlitePool,
        queue: Arc<Mutex<Vec<ScraperNodeInfo>>>,
    ) -> Result<()> {
        let nodes = get_nodes_for_scraping(pool).await?;
        tracing::info!("Querying {} mixing nodes", nodes.len());
        if let Ok(mut queue_lock) = queue.lock() {
            queue_lock.extend(nodes);
        } else {
            warn!("Failed to acquire packet queue lock");
            return Ok(());
        }

        Self::process_packet_queue(pool, queue).await;
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
                            task_id, node.node_id()
                        ),
                        Err(e) => debug!(
                            "📝 ❌ Description task #{} for node {} failed: {}",
                            task_id, node.node_id(), e
                        ),
                    }
                    TASK_COUNTER.fetch_sub(1, Ordering::Relaxed);
                });
            } else {
                tokio::time::sleep(QUEUE_CHECK_INTERVAL).await;
            }
        }
    }

    async fn process_packet_queue(pool: &SqlitePool, queue: Arc<Mutex<Vec<ScraperNodeInfo>>>) {
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
                        warn!("Failed to acquire packet queue lock");
                        break;
                    }
                };

                TASK_COUNTER.fetch_add(1, Ordering::Relaxed);
                let task_id = TASK_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
                let pool = pool.clone();

                tokio::spawn(async move {
                    match scrape_and_store_packet_stats(&pool, &node).await {
                        Ok(_) => debug!(
                            "📊 ✅ Packet stats task #{} for node {} complete",
                            task_id, node.node_id()
                        ),
                        Err(e) => debug!(
                            "📊 ❌ Packet stats task #{} for node {} failed: {}",
                            task_id, node.node_id(), e
                        ),
                    }
                    TASK_COUNTER.fetch_sub(1, Ordering::Relaxed);
                });
            } else {
                tokio::time::sleep(QUEUE_CHECK_INTERVAL).await;
            }
        }
    }
}
