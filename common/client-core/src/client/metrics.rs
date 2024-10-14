//! # Metrics collection and reporting.
//!
//! Modular metrics collection and reporting system. submodules can be added to collect different types of metrics.
//! On creation the Metrics controller will start a task that will listen for incoming stats events and
//! multiplex them out to the appropriate metrics module based on type.
//!
//! Adding A new module you need to write a new module that implements the `MetricsObj` trait and add it to
//! the `stats` hashmap in the `MetricsController` struct during it's initialization in the `new` function in
//! this file.

use std::{collections::HashMap, time::Duration};

// Metrics server
use futures::future::{FusedFuture, OptionFuture};
use futures::FutureExt;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use http_body_util::Full;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use hyper::body::Bytes;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use hyper::server::conn::http1;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use hyper::service::service_fn;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use hyper::{Request, Response};
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use hyper_util::rt::TokioIo;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::convert::Infallible;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[cfg(feature = "metrics-server")]
use std::net::SocketAddr;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use tokio::net::TcpListener;

use crate::spawn_future;

pub(crate) mod api_statistics;
pub(crate) mod gateway_conn_statistics;
pub(crate) mod packet_statistics;

// Time interval between reporting packet statistics
const PACKET_REPORT_INTERVAL_SECS: u64 = 2;
// Interval for taking snapshots of the packet statistics
const SNAPSHOT_INTERVAL_MS: u64 = 500;

#[derive(PartialEq, Eq, Hash, Debug)]
pub(crate) enum MetricsType {
    PacketStatistics,
    GatewayMetrics,
    APIMetrics,
}

pub(crate) enum MetricsEvents {
    PacketStatisticsEvent(packet_statistics::PacketStatisticsEvent),
    GatewayMetricsEvent(gateway_conn_statistics::GatewayMetricsEvent),
    APIMetricsEvent(api_statistics::APIMetricsEvent),
}

impl MetricsEvents {
    pub(crate) fn metrics_type(&self) -> MetricsType {
        match self {
            MetricsEvents::PacketStatisticsEvent(_) => MetricsType::PacketStatistics,
            MetricsEvents::GatewayMetricsEvent(_) => MetricsType::GatewayMetrics,
            MetricsEvents::APIMetricsEvent(_) => MetricsType::APIMetrics,
        }
    }
}

type MetricsReceiver = tokio::sync::mpsc::UnboundedReceiver<MetricsEvents>;

#[derive(Clone)]
pub(crate) struct MetricsSender {
    stats_tx: tokio::sync::mpsc::UnboundedSender<MetricsEvents>,
}

impl MetricsSender {
    pub(crate) fn new(stats_tx: tokio::sync::mpsc::UnboundedSender<MetricsEvents>) -> Self {
        MetricsSender { stats_tx }
    }

    pub(crate) fn report(&self, event: MetricsEvents) {
        if let Err(err) = self.stats_tx.send(event) {
            log::error!("Failed to send stats event: {:?}", err);
        }
    }
}

pub(crate) trait MetricsObj: MetricsReporter + Send {
    fn new() -> Self
    where
        Self: Sized;

    fn type_identity(&self) -> MetricsType;

    /// Handle an incoming stats event
    fn handle_event(&mut self, event: MetricsEvents);

    /// snapshot the current state of the metrics if the module wishes to use it
    fn snapshot(&mut self);

    /// Reset the metrics to their initial state.
    ///
    /// Used to periodically reset the metrics in accordance with periodic reporting strategy
    fn periodic_reset(&mut self);
}

/// This trait represents objects that can be reported by the metrics controller and
/// provides the function by which they will be called to report their metrics.
pub(crate) trait MetricsReporter {
    /// Marshall the metrics into a string and write them to the provided formatter.
    fn marshall(&self) -> std::io::Result<String>;
}

/// Launches and manages metrics collection and reporting.
///
/// This is designed to be generic to allow for multiple types of metrics to be collected and
/// reported.
pub(crate) struct MetricsController {
    /// Keep store the different types of metrics collectors
    stats: HashMap<MetricsType, Box<dyn MetricsObj>>,

    /// Incoming packet stats events from other tasks
    stats_rx: MetricsReceiver,
}

impl MetricsController {
    pub(crate) fn new() -> (Self, MetricsSender) {
        let (stats_tx, stats_rx) = tokio::sync::mpsc::unbounded_channel();

        let mut stats: HashMap<MetricsType, Box<dyn MetricsObj>> = HashMap::new();
        stats.insert(
            MetricsType::PacketStatistics,
            Box::new(packet_statistics::PacketStatisticsControl::new()),
        );

        stats.insert(
            MetricsType::GatewayMetrics,
            Box::new(gateway_conn_statistics::GatewayMetricsControl::new()),
        );
        stats.insert(
            MetricsType::APIMetrics,
            Box::new(api_statistics::APIMetricsControl::new()),
        );

        let metrics_sender = MetricsSender::new(stats_tx);

        (MetricsController { stats, stats_rx }, metrics_sender)
    }

    pub(crate) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        log::debug!("Started PacketStatisticsControl with graceful shutdown support");

        let report_interval = Duration::from_secs(PACKET_REPORT_INTERVAL_SECS);
        let mut report_interval = tokio::time::interval(report_interval);
        let snapshot_interval = Duration::from_millis(SNAPSHOT_INTERVAL_MS);
        let mut snapshot_interval = tokio::time::interval(snapshot_interval);

        cfg_if::cfg_if! {
            if #[cfg(all(target_arch = "wasm32", target_os = "unknown"))] {
                log::warn!("Metrics server is not supported on wasm32-unknown-unknown");
                let listener: Option<WasmEmpty> = None;
            } else if #[cfg(feature = "metrics-server")] {
                let mut metrics_port = 18000;
                let listener: Option<TcpListener>;
                loop {
                    let addr = SocketAddr::from(([0, 0, 0, 0], metrics_port));
                    match TcpListener::bind(addr).await {
                        Ok(l) => {
                            log::info!("###############################");
                            log::info!("Metrics endpoint is at: {:?}", l.local_addr());
                            log::info!("###############################");
                            listener = Some(l);
                            break;
                        },
                        Err(err) => {
                            log::warn!("Failed to bind metrics server: {:?}", err);
                            metrics_port += 1;
                        }
                    };
                }
            } else {
                log::info!("Metrics server is disabled!");
                let listener: Option<TcpListener> = None;
            }
        }

        loop {
            // it seems at some point tokio changed its select precondition evaluation,
            // and it's no longer checked before the future is evaluated.
            let accept_future: OptionFuture<_> = listener
                .as_ref()
                .map(|l| l.accept())
                .map(FutureExt::fuse)
                .into();

            tokio::select! {
                stats_event = self.stats_rx.recv() => match stats_event {
                    Some(stats_event) => {
                        log::trace!("MetricsController: Received stats event");
                        match self.stats.get_mut(&stats_event.metrics_type()) {
                            Some(stats) => stats.handle_event(stats_event),
                            None => log::warn!("received event for unregistered metrics type: {:?}", stats_event.metrics_type()),
                        }
                    },
                    None => {
                        log::trace!("PacketStatisticsControl: stopping since stats channel was closed");
                        break;
                    }
                },
                // conditional will disable the branch if we're in wasm32-unknown-unknown
                // use `_` to calm down clippy when running for wasm
                _result = accept_future, if !accept_future.is_terminated() => {
                    cfg_if::cfg_if! {
                        if #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))] {
                            if let Some(Ok((stream, _))) = _result {
                                let io = TokioIo::new(stream);

                                tokio::task::spawn(async move {
                                    if let Err(err) = http1::Builder::new()
                                        .serve_connection(io, service_fn(serve_metrics))
                                        .await
                                    {
                                        log::warn!("Error serving connection: {:?}", err);
                                    }
                                });
                            } else {
                                log::warn!("Error accepting connection");
                            }
                        }
                    }
                }
                _ = snapshot_interval.tick() => {
                    for (_type, stats) in &mut self.stats {
                        stats.snapshot();
                    }
                }
                _ = report_interval.tick() => {
                    self.report_all();
                }
                _ = shutdown.recv_with_delay() => {
                    log::trace!("PacketStatisticsControl: Received shutdown");
                    break;
                },
            }
        }
        log::debug!("PacketStatisticsControl: Exiting");
    }

    pub(crate) fn report_all(&mut self) {
        for (_type, stats) in &mut self.stats {
            match stats.marshall() {
                Ok(metrics) => log::info!(" {:?}: {:?}", stats.type_identity(), metrics),
                Err(err) => log::error!("{:?}: marshall metrics: {:?}", stats.type_identity(), err),
            }
            stats.periodic_reset();
        }
    }

    pub(crate) fn start_with_shutdown(mut self, task_client: nym_task::TaskClient) {
        spawn_future(async move {
            self.run_with_shutdown(task_client).await;
        })
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
async fn serve_metrics(
    _: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    use nym_metrics::metrics;

    Ok(Response::new(Full::new(Bytes::from(metrics!()))))
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
struct WasmEmpty;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl WasmEmpty {
    async fn accept(&self) {}
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use super::*;
    use crate::client::metrics::api_statistics::APIMetricsEvent;
    use crate::client::metrics::gateway_conn_statistics::GatewayMetricsEvent;
    use crate::client::metrics::packet_statistics::PacketStatisticsEvent;

    #[tokio::test]
    async fn test_metrics_controller() {
        let _ = pretty_env_logger::try_init();

        log::info!("print me please");
        let (metrics_controller, metrics_sender) = MetricsController::new();
        let m = Arc::new(Mutex::new(metrics_controller));
        let m1 = Arc::clone(&m);
        tokio::spawn(async move {
            let mut mc = m1.lock().await;
            mc.run_with_shutdown(nym_task::TaskClient::dummy()).await;
        });

        for _ in 0..10 {
            metrics_sender.report(MetricsEvents::PacketStatisticsEvent(
                PacketStatisticsEvent::RealPacketSent(1),
            ));
            metrics_sender.report(MetricsEvents::GatewayMetricsEvent(
                GatewayMetricsEvent::RealPacketSent(2),
            ));
            metrics_sender.report(MetricsEvents::APIMetricsEvent(
                APIMetricsEvent::RealPacketSent(3),
            ));
            tokio::time::sleep(Duration::from_secs(3)).await;
        }

        // assert_eq!(&m.lock().await.stats.get(&MetricsType::APIStatistics).unwrap().marshall().unwrap(), "PacketStatistics { sent: 3, received: 0 }");
        m.lock().await.report_all();
    }
}
