// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_metrics::{metrics_registry, Metric};
use std::sync::LazyLock;
use strum::{Display, EnumCount, EnumIter, EnumProperty, IntoEnumIterator};

pub static PROMETHEUS_METRICS: LazyLock<NymNodePrometheusMetrics> =
    LazyLock::new(|| NymNodePrometheusMetrics::initialise());

const CLIENT_SESSION_DURATION_BUCKETS: &[f64] = &[
    // sub 3s (implicitly)
    3.,      // 3s - 15s
    15.,     // 15s - 70s
    70.,     // 70s - 2min
    120.,    // 2 min - 5 min
    300.,    // 5min - 15min
    900.,    // 15min - 1h
    3600.,   // 1h - 12h
    43200.,  // 12h - 23.5h
    88200.,  // 23.5h - 24.5h
    86400.,  // 24.5h - 72h
    259200., // 72h+ (implicitly)
];

#[derive(Clone, Debug, EnumIter, Display, EnumProperty, EnumCount, Eq, Hash, PartialEq)]
#[strum(serialize_all = "snake_case", prefix = "nym_node_")]
pub enum PrometheusMetric {
    // # MIXNET
    // ## INGRESS
    #[strum(props(help = "The number of ingress forward hop sphinx packets received"))]
    MixnetIngressForwardPacketsReceived,

    #[strum(props(help = "The number of ingress final hop sphinx packets received"))]
    MixnetIngressFinalHopPacketsReceived,

    #[strum(props(help = "The number of ingress malformed sphinx packets received"))]
    MixnetIngressMalformedPacketsReceived,

    #[strum(props(
        help = "The number of ingress forward sphinx packets that specified excessive delay received"
    ))]
    MixnetIngressExcessiveDelayPacketsReceived,

    #[strum(props(help = "The number of ingress forward hop sphinx packets dropped"))]
    MixnetIngressForwardPacketsDropped,

    #[strum(props(help = "The number of ingress final hop sphinx packets dropped"))]
    MixnetIngressFinalHopPacketsDropped,

    #[strum(props(help = "The current rate of receiving ingress forward hop sphinx packets"))]
    MixnetIngressForwardPacketsReceivedRate,

    #[strum(props(help = "The current rate of receiving ingress final hop sphinx packets"))]
    MixnetIngressFinalHopPacketsReceivedRate,

    #[strum(props(help = "The current rate of receiving ingress malformed sphinx packets"))]
    MixnetIngressMalformedPacketsReceivedRate,

    #[strum(props(
        help = "The current rate of receiving ingress sphinx packets that specified excessive delay"
    ))]
    MixnetIngressExcessiveDelayPacketsReceivedRate,

    #[strum(props(help = "The current rate of dropping ingress forward hop sphinx packets"))]
    MixnetIngressForwardPacketsDroppedRate,

    #[strum(props(help = "The current rate of dropping ingress final hop sphinx packets"))]
    MixnetIngressFinalHopPacketsDroppedRate,

    // ## EGRESS
    #[strum(props(help = "The number of egress forward hop sphinx packets sent/forwarded"))]
    MixnetEgressForwardPacketsSent,

    #[strum(props(
        help = "The number of egress forward hop sphinx packets sent/forwarded (acks only)"
    ))]
    MixnetEgressAckSent,

    #[strum(props(help = "The number of egress forward hop sphinx packets dropped"))]
    MixnetEgressForwardPacketsDropped,

    #[strum(props(
        help = "The current rate of sending/forwarding egress forward hop sphinx packets"
    ))]
    MixnetEgressForwardPacketsSentRate,

    #[strum(props(
        help = "The current rate of sending/forwarding egress forward hop sphinx packets (acks only)"
    ))]
    MixnetEgressAckSentRate,

    #[strum(props(help = "The current rate of dropping egress forward hop sphinx packets"))]
    MixnetEgressForwardPacketsDroppedRate,

    // # ENTRY
    #[strum(props(help = "The number of unique users"))]
    EntryClientUniqueUsers,

    #[strum(props(help = "The number of client sessions started"))]
    EntryClientSessionsStarted,

    #[strum(props(help = "The number of client sessions finished"))]
    EntryClientSessionsFinished,

    #[strum(to_string = "entry_client_sessions_durations_{typ}")]
    #[strum(props(help = "The distribution of client sessions duration of the specified type"))]
    EntryClientSessionsDurations { typ: String },

    // # WIREGUARD
    #[strum(props(help = "The amount of bytes transmitted via wireguard"))]
    WireguardBytesTx,

    #[strum(props(help = "The amount of bytes received via wireguard"))]
    WireguardBytesRx,

    #[strum(props(help = "The current number of all registered wireguard peers"))]
    WireguardTotalPeers,

    #[strum(props(help = "The current number of active wireguard peers"))]
    WireguardActivePeers,

    #[strum(props(help = "The current sending rate of wireguard"))]
    WireguardBytesTxRate,

    #[strum(props(help = "The current receiving rate of wireguard"))]
    WireguardBytesRxRate,

    // # NETWORK
    #[strum(props(help = "The number of active ingress mixnet connections"))]
    NetworkActiveIngressMixnetConnections,

    #[strum(props(help = "The number of active ingress websocket connections"))]
    NetworkActiveIngressWebSocketConnections,

    #[strum(props(help = "The number of active egress mixnet connections"))]
    NetworkActiveEgressMixnetConnections,
}

impl PrometheusMetric {
    fn name(&self) -> String {
        self.to_string()
    }

    fn help(&self) -> &'static str {
        // SAFETY: every variant has a `help` prop defined (and there's a unit test is checking for that)
        #[allow(clippy::unwrap_used)]
        self.get_str("help").unwrap()
    }

    fn is_complex(&self) -> bool {
        match self {
            PrometheusMetric::EntryClientSessionsDurations { .. } => true,
            _ => false,
        }
    }

    fn to_registrable_metric(&self) -> Option<Metric> {
        let name = self.name();
        let help = self.help();

        match self {
            PrometheusMetric::MixnetIngressForwardPacketsReceived => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::MixnetIngressFinalHopPacketsReceived => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::MixnetIngressMalformedPacketsReceived => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::MixnetIngressExcessiveDelayPacketsReceived => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::MixnetIngressForwardPacketsDropped => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::MixnetIngressFinalHopPacketsDropped => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::MixnetIngressForwardPacketsReceivedRate => {
                Metric::new_float_gauge(&name, help)
            }
            PrometheusMetric::MixnetIngressFinalHopPacketsReceivedRate => {
                Metric::new_float_gauge(&name, help)
            }
            PrometheusMetric::MixnetIngressMalformedPacketsReceivedRate => {
                Metric::new_float_gauge(&name, help)
            }
            PrometheusMetric::MixnetIngressExcessiveDelayPacketsReceivedRate => {
                Metric::new_float_gauge(&name, help)
            }
            PrometheusMetric::MixnetIngressForwardPacketsDroppedRate => {
                Metric::new_float_gauge(&name, help)
            }
            PrometheusMetric::MixnetIngressFinalHopPacketsDroppedRate => {
                Metric::new_float_gauge(&name, help)
            }
            PrometheusMetric::MixnetEgressForwardPacketsSent => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::MixnetEgressAckSent => Metric::new_int_counter(&name, help),
            PrometheusMetric::MixnetEgressForwardPacketsDropped => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::MixnetEgressForwardPacketsSentRate => {
                Metric::new_float_gauge(&name, help)
            }
            PrometheusMetric::MixnetEgressAckSentRate => Metric::new_float_gauge(&name, help),
            PrometheusMetric::MixnetEgressForwardPacketsDroppedRate => {
                Metric::new_float_gauge(&name, help)
            }
            PrometheusMetric::EntryClientUniqueUsers => Metric::new_int_counter(&name, help),
            PrometheusMetric::EntryClientSessionsStarted => Metric::new_int_counter(&name, help),
            PrometheusMetric::EntryClientSessionsFinished => Metric::new_int_counter(&name, help),
            PrometheusMetric::EntryClientSessionsDurations { .. } => {
                Metric::new_histogram(&name, help, Some(&CLIENT_SESSION_DURATION_BUCKETS))
            }
            PrometheusMetric::WireguardBytesTx => Metric::new_int_counter(&name, help),
            PrometheusMetric::WireguardBytesRx => Metric::new_int_counter(&name, help),
            PrometheusMetric::WireguardTotalPeers => Metric::new_int_counter(&name, help),
            PrometheusMetric::WireguardActivePeers => Metric::new_int_counter(&name, help),
            PrometheusMetric::WireguardBytesTxRate => Metric::new_float_gauge(&name, help),
            PrometheusMetric::WireguardBytesRxRate => Metric::new_float_gauge(&name, help),
            PrometheusMetric::NetworkActiveIngressMixnetConnections => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::NetworkActiveIngressWebSocketConnections => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::NetworkActiveEgressMixnetConnections => {
                Metric::new_int_counter(&name, help)
            }
        }
    }

    fn set(&self, value: i64) {
        metrics_registry().set(&self.name(), value);
    }

    fn set_float(&self, value: f64) {
        metrics_registry().set_float(&self.name(), value);
    }

    fn inc(&self) {
        metrics_registry().inc(&self.name());
    }

    fn inc_by(&self, value: i64) {
        metrics_registry().inc_by(&self.name(), value);
    }

    fn observe_histogram(&self, value: f64) {
        let reg = metrics_registry();
        if !reg.add_to_histogram(&self.name(), value) {
            if let Some(registrable) = self.to_registrable_metric() {
                reg.register_metric(registrable);
                reg.add_to_histogram(&self.name(), value);
            }
        }
    }
}

#[non_exhaustive]
pub struct NymNodePrometheusMetrics {}

impl NymNodePrometheusMetrics {
    // initialise all fields on startup with default values so that they'd be immediately available for query
    pub(crate) fn initialise() -> Self {
        let registry = metrics_registry();

        // we can't initialise complex metrics as their names will only be fully known at runtime
        for kind in PrometheusMetric::iter() {
            if !kind.is_complex() {
                if let Some(metric) = kind.to_registrable_metric() {
                    registry.register_metric(metric);
                }
            }
        }

        NymNodePrometheusMetrics {}
    }

    pub fn set(&self, metric: PrometheusMetric, value: i64) {
        metric.set(value)
    }

    pub fn set_float(&self, metric: PrometheusMetric, value: f64) {
        metric.set_float(value)
    }

    pub fn inc(&self, metric: PrometheusMetric) {
        metric.inc()
    }

    pub fn inc_by(&self, metric: PrometheusMetric, value: i64) {
        metric.inc_by(value)
    }

    pub fn observe_histogram(&self, metric: PrometheusMetric, value: f64) {
        metric.observe_histogram(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn prometheus_metrics() {
        // a sanity check for anyone adding new metrics. if this test fails,
        // make sure any methods on `PrometheusMetric` enum don't need updating
        // or require custom Display impl
        assert_eq!(31, PrometheusMetric::COUNT)
    }

    #[test]
    fn every_variant_has_help_property() {
        for variant in PrometheusMetric::iter() {
            assert!(variant.get_str("help").is_some())
        }
    }

    #[test]
    fn prometheus_metrics_names() {
        // make sure nothing changed in our serialisation
        let simple = PrometheusMetric::MixnetIngressForwardPacketsReceived.to_string();
        assert_eq!("nym_node_mixnet_ingress_forward_packets_received", simple);

        let parameterised =
            PrometheusMetric::EntryClientSessionsDurations { typ: "vpn".into() }.to_string();
        assert_eq!(
            "nym_node_entry_client_sessions_durations_vpn",
            parameterised
        )
    }
}
