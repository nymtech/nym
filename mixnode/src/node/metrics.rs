use directory_client::metrics::MixMetric;
use directory_client::requests::metrics_mixes_post::MetricsMixPoster;
use directory_client::DirectoryClient;
use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::StreamExt;
use log::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

type SentMetricsMap = HashMap<String, u64>;

pub(crate) enum MetricEvent {
    Sent(String),
    Received,
}

#[derive(Debug, Clone)]
// Note: you should NEVER create more than a single instance of this using 'new()'.
// You should always use .clone() to create additional instances
struct MixMetrics {
    inner: Arc<Mutex<MixMetricsInner>>,
}

struct MixMetricsInner {
    received: u64,
    sent: SentMetricsMap,
}

impl MixMetrics {
    pub(crate) fn new() -> Self {
        MixMetrics {
            inner: Arc::new(Mutex::new(MixMetricsInner {
                received: 0,
                sent: HashMap::new(),
            })),
        }
    }

    async fn increment_received_metrics(&mut self) {
        let mut unlocked = self.inner.lock().await;
        unlocked.received += 1;
    }

    async fn increment_sent_metrics(&mut self, destination: String) {
        let mut unlocked = self.inner.lock().await;
        let receiver_count = unlocked.sent.entry(destination).or_insert(0);
        *receiver_count += 1;
    }

    async fn acquire_and_reset_metrics(&mut self) -> (u64, SentMetricsMap) {
        let mut unlocked = self.inner.lock().await;
        let received = unlocked.received;

        let sent = std::mem::replace(&mut unlocked.sent, HashMap::new());
        unlocked.received = 0;

        (received, sent)
    }
}

struct MetricsReceiver {
    metrics: MixMetrics,
    metrics_rx: mpsc::UnboundedReceiver<MetricEvent>,
}

impl MetricsReceiver {
    fn new(metrics: MixMetrics, metrics_rx: mpsc::UnboundedReceiver<MetricEvent>) -> Self {
        MetricsReceiver {
            metrics,
            metrics_rx,
        }
    }

    fn start(mut self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
            while let Some(metrics_data) = self.metrics_rx.next().await {
                match metrics_data {
                    MetricEvent::Received => self.metrics.increment_received_metrics().await,
                    MetricEvent::Sent(destination) => {
                        self.metrics.increment_sent_metrics(destination).await
                    }
                }
            }
        })
    }
}

struct MetricsSender {
    metrics: MixMetrics,
    directory_client: directory_client::Client,
    pub_key_str: String,
    sending_delay: Duration,
    metrics_informer: MetricsInformer,
}

impl MetricsSender {
    fn new(
        metrics: MixMetrics,
        directory_server: String,
        pub_key_str: String,
        sending_delay: Duration,
    ) -> Self {
        MetricsSender {
            metrics,
            directory_client: directory_client::Client::new(directory_client::Config::new(
                directory_server,
            )),
            pub_key_str,
            sending_delay,
            metrics_informer: MetricsInformer::new(),
        }
    }

    fn start(mut self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
            loop {
                // set the deadline in the future
                let sending_delay = tokio::time::delay_for(self.sending_delay);
                let (received, sent) = self.metrics.acquire_and_reset_metrics().await;

                self.metrics_informer.log_stats(received, &sent);
                match self.directory_client.metrics_post.post(&MixMetric {
                    pub_key: self.pub_key_str.clone(),
                    received,
                    sent,
                }) {
                    Err(err) => error!("failed to send metrics - {:?}", err),
                    Ok(_) => debug!("sent metrics information"),
                }

                // wait for however much is left
                sending_delay.await;
            }
        })
    }
}

#[derive(Default)]
struct MetricsInformer {
    total_received: u64,
    sent_map: SentMetricsMap,
}

impl MetricsInformer {
    fn new() -> Self {
        Default::default()
    }

    fn update_runnings_stats(&mut self, pre_reset_received: u64, pre_reset_sent: &SentMetricsMap) {
        self.total_received += pre_reset_received;

        for (mix, count) in pre_reset_sent.iter() {
            *self.sent_map.entry(mix.clone()).or_insert(0) += *count;
        }
    }

    fn log_report_stats(&self, pre_reset_received: u64, pre_reset_sent: &SentMetricsMap) {
        info!(
            "Since last metrics report mixed {} packets!",
            pre_reset_received
        );
        debug!(
            "Since last metrics report received {} packets",
            pre_reset_sent.values().sum::<u64>()
        );
        trace!(
            "Since last metrics report sent packets to the following: \n{:#?}",
            pre_reset_sent
        );
    }

    fn log_running_stats(&self) {
        info!(
            "Since startup mixed {} packets!",
            self.sent_map.values().sum::<u64>()
        );
        debug!("Since startup received {} packets", self.total_received);
        trace!(
            "Since startup sent packets to the following: \n{:#?}",
            self.sent_map
        );
    }

    fn log_stats(&mut self, pre_reset_received: u64, pre_reset_sent: &SentMetricsMap) {
        self.update_runnings_stats(pre_reset_received, pre_reset_sent);

        self.log_report_stats(pre_reset_received, pre_reset_sent);
        self.log_running_stats()
    }
}

#[derive(Clone)]
pub struct MetricsReporter {
    metrics_tx: mpsc::UnboundedSender<MetricEvent>,
}

impl MetricsReporter {
    pub(crate) fn new(metrics_tx: mpsc::UnboundedSender<MetricEvent>) -> Self {
        MetricsReporter { metrics_tx }
    }

    pub(crate) fn report_sent(&self, destination: String) {
        // in unbounded_send() failed it means that the receiver channel was disconnected
        // and hence something weird must have happened without a way of recovering
        self.metrics_tx
            .unbounded_send(MetricEvent::Sent(destination))
            .unwrap()
    }

    pub(crate) fn report_received(&self) {
        // in unbounded_send() failed it means that the receiver channel was disconnected
        // and hence something weird must have happened without a way of recovering
        self.metrics_tx
            .unbounded_send(MetricEvent::Received)
            .unwrap()
    }
}

// basically an easy single entry point to start all metrics related tasks
pub struct MetricsController {
    receiver: MetricsReceiver,
    reporter: MetricsReporter,
    sender: MetricsSender,
}

impl MetricsController {
    pub(crate) fn new(
        directory_server: String,
        pub_key_str: String,
        sending_delay: Duration,
    ) -> Self {
        let (metrics_tx, metrics_rx) = mpsc::unbounded();
        let shared_metrics = MixMetrics::new();

        MetricsController {
            sender: MetricsSender::new(
                shared_metrics.clone(),
                directory_server,
                pub_key_str,
                sending_delay,
            ),
            receiver: MetricsReceiver::new(shared_metrics, metrics_rx),
            reporter: MetricsReporter::new(metrics_tx),
        }
    }

    // reporter is how node is going to be accessing the metrics data
    pub(crate) fn start(self, handle: &Handle) -> MetricsReporter {
        // TODO: should we do anything with JoinHandle(s) returned by start methods?
        self.receiver.start(handle);
        self.sender.start(handle);
        self.reporter
    }
}
