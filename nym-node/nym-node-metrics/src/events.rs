// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use futures::channel::mpsc::SendError;
pub use nym_statistics_common::gateways::GatewaySessionEvent;
use tracing::error;

pub fn events_channels() -> (MetricEventsSender, MetricEventsReceiver) {
    let (tx, rx) = mpsc::unbounded();
    (tx.into(), rx)
}

#[derive(Clone)]
pub struct MetricEventsSender(mpsc::UnboundedSender<MetricsEvent>);

impl From<mpsc::UnboundedSender<MetricsEvent>> for MetricEventsSender {
    fn from(tx: mpsc::UnboundedSender<MetricsEvent>) -> Self {
        MetricEventsSender(tx)
    }
}

impl MetricEventsSender {
    pub fn report(&self, metric: impl Into<MetricsEvent>) -> Result<(), SendError> {
        self.0
            .unbounded_send(metric.into())
            .map_err(|err| err.into_send_error())
    }

    pub fn report_unchecked(&self, metric: impl Into<MetricsEvent>) {
        if let Err(err) = self.report(metric) {
            error!("failed to send metric information: {err}")
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

pub type MetricEventsReceiver = mpsc::UnboundedReceiver<MetricsEvent>;

// please create a new variant per "category" of metrics
pub enum MetricsEvent {
    GatewayClientSession(GatewaySessionEvent),
}

impl From<GatewaySessionEvent> for MetricsEvent {
    fn from(gateway_stats: GatewaySessionEvent) -> Self {
        MetricsEvent::GatewayClientSession(gateway_stats)
    }
}
