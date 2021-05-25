// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::StreamExt;
use log::trace;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::task::JoinHandle;

const METRICS_FAILURE_BACKOFF: Duration = Duration::from_secs(30);

type SentMetricsMap = HashMap<String, u64>;

pub(crate) enum MetricEvent {
    Sent(String),
    Received,
}

#[derive(Debug, Clone)]
// Note: you should NEVER create more than a single instance of this using 'new()'.
// You should always use .clone() to create additional instances
struct MixMetrics {
    inner: Arc<MixMetricsInner>,
}

#[derive(Debug)]
struct MixMetricsInner {
    received: AtomicU64,
    sent: Mutex<SentMetricsMap>,
}

impl MixMetrics {
    pub(crate) fn new() -> Self {
        MixMetrics {
            inner: Arc::new(MixMetricsInner {
                received: AtomicU64::new(0),
                sent: Mutex::new(HashMap::new()),
            }),
        }
    }

    fn increment_received_metrics(&mut self) {
        self.inner.received.fetch_add(1, Ordering::SeqCst);
    }

    async fn increment_sent_metrics(&mut self, destination: String) {
        let mut unlocked = self.inner.sent.lock().await;
        let receiver_count = unlocked.entry(destination).or_insert(0);
        *receiver_count += 1;
    }

    async fn acquire_and_reset_metrics(&mut self) -> (u64, SentMetricsMap) {
        let mut unlocked = self.inner.sent.lock().await;
        let received = self.inner.received.swap(0, Ordering::SeqCst);

        let sent = std::mem::take(unlocked.deref_mut());

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

    fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(metrics_data) = self.metrics_rx.next().await {
                match metrics_data {
                    MetricEvent::Received => self.metrics.increment_received_metrics(),
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
    #[allow(dead_code)]
    metrics_client: metrics_client::Client,
    #[allow(dead_code)]
    pub_key_str: String,
    metrics_informer: MetricsInformer,
}

impl MetricsSender {
    fn new(
        metrics: MixMetrics,
        metrics_server: String,
        pub_key_str: String,
        running_logging_delay: Duration,
    ) -> Self {
        MetricsSender {
            metrics,
            metrics_client: metrics_client::Client::new(metrics_client::Config::new(
                metrics_server,
            )),
            pub_key_str,
            metrics_informer: MetricsInformer::new(running_logging_delay),
        }
    }

    fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let (received, sent) = self.metrics.acquire_and_reset_metrics().await;

                self.metrics_informer.update_running_stats(received, &sent);
                self.metrics_informer.log_report_stats(received, &sent);
                self.metrics_informer.try_log_running_stats();

                tokio::time::sleep(METRICS_FAILURE_BACKOFF).await;

                // let sending_delay = match self
                //     .metrics_client
                //     .post_mix_metrics(MixMetric {
                //         pub_key: self.pub_key_str.clone(),
                //         received,
                //         sent,
                //     })
                //     .await
                // {
                //     Err(err) => {
                //         error!("failed to send metrics - {:?}", err);
                //         tokio::time::sleep(METRICS_FAILURE_BACKOFF)
                //     }
                //     Ok(new_interval) => {
                //         debug!("sent metrics information");
                //         tokio::time::sleep(Duration::from_secs(new_interval.next_report_in))
                //     }
                // };
                //
                // // wait for however much is left
                // sending_delay.await;
            }
        })
    }
}

struct MetricsInformer {
    total_received: u64,
    sent_map: SentMetricsMap,

    running_stats_logging_delay: Duration,
    last_reported_stats: SystemTime,
}

impl MetricsInformer {
    fn new(running_stats_logging_delay: Duration) -> Self {
        MetricsInformer {
            total_received: 0,
            sent_map: HashMap::new(),
            running_stats_logging_delay,
            last_reported_stats: SystemTime::now(),
        }
    }

    fn should_log_running_stats(&self) -> bool {
        self.last_reported_stats + self.running_stats_logging_delay < SystemTime::now()
    }

    fn try_log_running_stats(&mut self) {
        if self.should_log_running_stats() {
            self.log_running_stats()
        }
    }

    fn update_running_stats(&mut self, pre_reset_received: u64, pre_reset_sent: &SentMetricsMap) {
        self.total_received += pre_reset_received;

        for (mix, count) in pre_reset_sent.iter() {
            *self.sent_map.entry(mix.clone()).or_insert(0) += *count;
        }
    }

    fn log_report_stats(&self, pre_reset_received: u64, pre_reset_sent: &SentMetricsMap) {
        debug!(
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

    fn log_running_stats(&mut self) {
        info!(
            "Since startup mixed {} packets!",
            self.sent_map.values().sum::<u64>()
        );
        debug!("Since startup received {} packets", self.total_received);
        trace!(
            "Since startup sent packets to the following: \n{:#?}",
            self.sent_map
        );
        self.last_reported_stats = SystemTime::now();
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

    // TODO: in the future this could be slightly optimised to get rid of the channel
    // in favour of incrementing value directly
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
        running_stats_logging_delay: Duration,
    ) -> Self {
        let (metrics_tx, metrics_rx) = mpsc::unbounded();
        let shared_metrics = MixMetrics::new();

        MetricsController {
            sender: MetricsSender::new(
                shared_metrics.clone(),
                directory_server,
                pub_key_str,
                running_stats_logging_delay,
            ),
            receiver: MetricsReceiver::new(shared_metrics, metrics_rx),
            reporter: MetricsReporter::new(metrics_tx),
        }
    }

    // reporter is how node is going to be accessing the metrics data
    pub(crate) fn start(self) -> MetricsReporter {
        // TODO: should we do anything with JoinHandle(s) returned by start methods?
        self.receiver.start();
        self.sender.start();
        self.reporter
    }
}
