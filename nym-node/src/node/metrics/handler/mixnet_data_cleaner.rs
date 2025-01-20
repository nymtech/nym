// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::metrics::handler::{
    MetricsHandler, OnStartMetricsHandler, OnUpdateMetricsHandler,
};
use async_trait::async_trait;
use nym_node_metrics::mixnet::{EgressRecipientStats, IngressRecipientStats};
use nym_node_metrics::NymNodeMetrics;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};

// it can be anything, we just need a unique type_id to register our handler
pub struct StaleMixnetMetrics;

#[derive(Default)]
pub struct LastSeen {
    ingress_senders: HashMap<IpAddr, IngressRecipientStats>,
    egress_forward_recipients: HashMap<SocketAddr, EgressRecipientStats>,
}

pub struct MixnetMetricsCleaner {
    metrics: NymNodeMetrics,
    last_seen: LastSeen,
}

impl MixnetMetricsCleaner {
    pub(crate) fn new(metrics: NymNodeMetrics) -> Self {
        MixnetMetricsCleaner {
            metrics,
            last_seen: LastSeen::default(),
        }
    }
}

#[async_trait]
impl OnStartMetricsHandler for MixnetMetricsCleaner {}

#[async_trait]
impl OnUpdateMetricsHandler for MixnetMetricsCleaner {
    async fn on_update(&mut self) {
        let mut senders_to_remove = Vec::new();
        let mut recipients_to_remove = Vec::new();

        for sender_entry in self.metrics.mixnet.ingress.senders().iter() {
            if let Some(last_seen) = self.last_seen.ingress_senders.get(sender_entry.key()) {
                if sender_entry.value() == last_seen {
                    senders_to_remove.push(*sender_entry.key());
                }
            }
        }

        for recipient_entry in self.metrics.mixnet.egress.forward_recipients().iter() {
            if let Some(last_seen) = self
                .last_seen
                .egress_forward_recipients
                .get(recipient_entry.key())
            {
                if recipient_entry.value() == last_seen {
                    recipients_to_remove.push(*recipient_entry.key());
                }
            }
        }

        // no need to make copies if data hasn't changed
        if !senders_to_remove.is_empty() {
            let mut new_ingress_senders = HashMap::new();

            for sender in senders_to_remove {
                self.metrics.mixnet.ingress.remove_stale_sender(sender)
            }

            for sender_entry in self.metrics.mixnet.ingress.senders() {
                new_ingress_senders.insert(*sender_entry.key(), *sender_entry.value());
            }

            self.last_seen.ingress_senders = new_ingress_senders;
        }

        if !recipients_to_remove.is_empty() {
            let mut new_egress_forward_recipients = HashMap::new();

            for recipient in recipients_to_remove {
                self.metrics
                    .mixnet
                    .egress
                    .remove_stale_forward_recipient(recipient)
            }

            for recipient_entry in self.metrics.mixnet.egress.forward_recipients() {
                new_egress_forward_recipients
                    .insert(*recipient_entry.key(), *recipient_entry.value());
            }

            self.last_seen.egress_forward_recipients = new_egress_forward_recipients;
        }
    }
}

#[async_trait]
impl MetricsHandler for MixnetMetricsCleaner {
    type Events = StaleMixnetMetrics;

    async fn handle_event(&mut self, _event: Self::Events) {
        panic!("this should have never been called! MetricsHandler has been incorrectly called on MixnetMetricsCleaner")
    }
}
