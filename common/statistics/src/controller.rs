use crate::{
    clients::{ClientStatsController, ClientStatsReceiver, ClientStatsSender, LocalStatsController}, report::{Sink, Sinks}, spawn_future, Runtime
};

use std::time::Duration;

use nym_client_core_config_types::StatsReporting;
// use nym_client_core::{client::inbound_messages::{InputMessage, InputMessageSender}, spawn_future };

/// Time interval between reporting statistics locally (logging/task_client)
const LOCAL_REPORT_INTERVAL: Duration = Duration::from_secs(10);
/// Interval for taking snapshots of the statistics
const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(500);

/// Launches and manages metrics collection and reporting.
///
/// This is designed to be generic to allow for multiple types of metrics to be collected and
/// reported.
pub struct StatisticsControl {
    /// Keep store the different types of metrics collectors
    stats: ClientStatsController,

    /// Keep store the different types of metrics collectors for internal tracking
    local_stats: LocalStatsController,

    /// Incoming packet stats events from other tasks
    stats_rx: ClientStatsReceiver,

    /// Channel to send stats report through the mixnet
    report: Sinks,

    /// Config for stats reporting (enabled, address, interval)
    reporting_config: StatsReporting,
}

impl StatisticsControl {
    pub fn create(
        reporting_config: StatsReporting,
        client_type: String,
        client_stats_id: String,
        report: Sinks,
    ) -> (Self, ClientStatsSender) {
        let (stats_tx, stats_rx) = tokio::sync::mpsc::unbounded_channel();

        let stats = ClientStatsController::new(client_stats_id, client_type);

        (
            StatisticsControl {
                stats,
                local_stats: Default::default(),
                stats_rx,
                report,
                reporting_config,
            },
            ClientStatsSender::new(Some(stats_tx)),
        )
    }

	/// Reports to potentially remote stats handler IFF they are enabled and configured
    async fn report_stats(&mut self) {
        let stats_report = self.stats.build_report();

		self.report.report(&stats_report.to_string()).await;
		self.stats.reset();
    }

	/// Reports to local stats handlers, logging, application facing handlers, etc.
	async fn report_local_stats(&mut self) {
        let stats_report = self.local_stats.build_report();

		self.report.local_report(&stats_report.to_string()).await;
		self.local_stats.reset();
    }

    async fn run_with_shutdown(&mut self, mut runtime: Runtime) {
        log::debug!("Started StatisticsControl with graceful shutdown support");

        let mut stats_report_interval =
            tokio::time::interval(self.reporting_config.reporting_interval);
        let mut local_report_interval = tokio::time::interval(LOCAL_REPORT_INTERVAL);
        let mut snapshot_interval = tokio::time::interval(SNAPSHOT_INTERVAL);

        loop {
            tokio::select! {
                stats_event = self.stats_rx.recv() => match stats_event {
                        Some(stats_event) => {
                            self.stats.handle_event(stats_event.clone());
                            self.local_stats.handle_event(stats_event);
                        }
                        None => {
                            log::trace!("StatisticsControl: shutting down due to closed stats channel");
                            break;
                        }
                },
                _ = snapshot_interval.tick() => {
                    self.stats.snapshot();
                }
                _ = stats_report_interval.tick(), if self.reporting_config.enabled && self.reporting_config.provider_address.is_some() => {
                    self.report_stats().await;
                }
                _ = local_report_interval.tick() => {
                    self.report_local_stats();
                }
                _ = runtime.cancelled() => {
                    log::trace!("StatisticsControl: Received shutdown");
                    break;
                },
            }
        }
        runtime.recv_timeout().await;
        log::debug!("StatisticsControl: Exiting");
    }

    pub fn start_with_shutdown(mut self, task_client: nym_task::TaskClient) {
        spawn_future(async move {
            self.run_with_shutdown(Runtime::Task(task_client)).await;
        })
    }

    pub fn create_and_start_with_shutdown(
        reporting_config: StatsReporting,
        client_type: String,
        client_stats_id: String,
        report: Sinks,
        task_client: nym_task::TaskClient,
    ) -> ClientStatsSender {
        let (controller, sender) =
            Self::create(reporting_config, client_type, client_stats_id, report);
        controller.start_with_shutdown(task_client);
        sender
    }
}
