use std::sync::Arc;

use chrono::{DateTime, Utc};
use log::{debug, error, info};
use rand::Rng;
use tokio::sync::broadcast::Receiver;
use tokio::sync::Mutex;
use tokio::time::Interval;

use super::metrics::types::MixnodeResult;
use super::storage::db::Storage;
use super::NR_OF_MIX_NODES;

pub mod types;

pub struct MetricsCollector {
    pub storage: Arc<Mutex<Storage>>,
    pub interval: Interval,
}

impl MetricsCollector {
    pub fn new(storage: Arc<Mutex<Storage>>, metrics_interval: i64) -> MetricsCollector {
        let interval =
            tokio::time::interval(std::time::Duration::from_secs(metrics_interval as u64));
        info!("Metrics collector interval: {:?}", metrics_interval);

        MetricsCollector { storage, interval }
    }

    pub(crate) async fn start(mut self, mut shutdown: Receiver<()>) {
        loop {
            tokio::select! {
                _ = self.interval.tick() => {
                    if let Err(e) = self.collect().await {
                        error!("Failed to collect metrics: {}", e);
                        break;
                    }
                }
                _ = shutdown.recv() => {
                    info!("Stopping metrics collector");
                    break;
                }
            }
        }
        info!("Metrics collector stopped")
    }

    pub(crate) async fn collect(&mut self) -> anyhow::Result<()> {
        let metrics = self.generate_metrics();
        let mut storage = self.storage.lock().await;

        let now: DateTime<Utc> = Utc::now();

        info!("Storing metrics for {} mixnodes, {}", NR_OF_MIX_NODES, now);
        storage.submit_mixnode_statuses(now.timestamp(), metrics)
    }

    fn generate_metrics(&self) -> Vec<MixnodeResult> {
        let mut metrics = Vec::with_capacity(NR_OF_MIX_NODES as usize);
        let mut rng = rand::thread_rng();

        for i in 0..NR_OF_MIX_NODES {
            let reliability = rng.gen_range(0..100) as u8;
            metrics.push(MixnodeResult {
                mix_id: i as u32,
                reliability,
            });
        }
        debug!("Generated metrics {:?}", metrics);
        metrics
    }
}
