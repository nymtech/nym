use std::sync::Arc;
use futures::lock::Mutex;
use futures::channel::mpsc;
use futures::StreamExt;
use std::time::Duration;
use nym_client::clients::directory;
use nym_client::clients::directory::DirectoryClient;
use nym_client::clients::directory::requests::metrics_mixes_post::MetricsMixPoster;
use nym_client::clients::directory::metrics::MixMetric;
use std::collections::HashMap;

const METRICS_INTERVAL: u64 = 3;

#[derive(Debug)]
pub struct MetricsReporter {
    received: u64,
    sent: HashMap<String, u64>
}

impl MetricsReporter {
    pub(crate) fn new() -> Self {
        MetricsReporter{
            received: 0,
            sent: HashMap::new(),
        }
    }

    pub(crate) fn add_arc_mutex(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }

    async fn increment_received_metrics(metrics: Arc<Mutex<MetricsReporter>>) {
        let mut unlocked = metrics.lock().await;
        unlocked.received += 1;
    }

    pub(crate) async fn run_received_metrics_control(metrics: Arc<Mutex<MetricsReporter>>, mut rx: mpsc::Receiver<()>) {
        while let Some(_) = rx.next().await {
            MetricsReporter::increment_received_metrics(metrics.clone()).await;
        }
    }

    async fn increment_sent_metrics(metrics: Arc<Mutex<MetricsReporter>>, sent_to: String) {
        let mut unlocked = metrics.lock().await;
        let receiver_count = unlocked.sent.entry(sent_to).or_insert(0);
        *receiver_count += 1;
    }

    pub(crate) async fn run_sent_metrics_control(metrics: Arc<Mutex<MetricsReporter>>, mut rx: mpsc::Receiver<String>) {
        while let Some(sent_metric) = rx.next().await {
            MetricsReporter::increment_sent_metrics(metrics.clone(), sent_metric).await;
        }
    }

    async fn acquire_and_reset_metrics(metrics: Arc<Mutex<MetricsReporter>>) -> (u64, HashMap<String, u64>) {
        let mut unlocked = metrics.lock().await;
        let received = unlocked.received;

        let sent = std::mem::replace(&mut unlocked.sent, HashMap::new());
        unlocked.received = 0;

        (received, sent)
    }

    pub(crate) async fn run_metrics_sender(metrics: Arc<Mutex<MetricsReporter>>, cfg: directory::Config, pub_key_str: String) {
        let delay_duration = Duration::from_secs(METRICS_INTERVAL);
        let directory_client= directory::Client::new(cfg);
        loop {
            tokio::time::delay_for(delay_duration).await;
            let (received, sent) = MetricsReporter::acquire_and_reset_metrics(metrics.clone()).await;
            directory_client.metrics_post.post(&MixMetric{
                pub_key: pub_key_str.clone(),
                received,
                sent,
            }).unwrap();
        }
    }
}
