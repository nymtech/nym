// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_geolocation_contract_common::payload::Location;
use nym_geolocation_contract_common::{GeolocationRecord, Subject};
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use nym_validator_client::nyxd::contract_traits::PagedGeolocationQueryClient;
use nym_validator_client::nyxd::nym_performance_contract_common::NodeId;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::error;

#[derive(Clone)]
pub(crate) struct SubmittedLocation {
    pub(crate) checked_at: OffsetDateTime,
    pub(crate) location: Location,
}

impl SubmittedLocation {
    pub(crate) fn has_expired(&self, now: OffsetDateTime, ttl: Duration) -> bool {
        now - self.checked_at > ttl
    }
}

pub(crate) struct OnChainNodes {
    nodes: Arc<RwLock<HashMap<NodeId, SubmittedLocation>>>,
}

impl OnChainNodes {
    pub(crate) async fn build_new(client: &DirectSigningHttpRpcNyxdClient) -> anyhow::Result<Self> {
        let addr = client.address();
        let records = client.get_all_geolocation_records().await?;

        let mut nodes = HashMap::new();
        for record in records {
            let GeolocationRecord::Location(location_record) = record else {
                continue;
            };
            if location_record.source.agent().map(|a| a.as_str()) != Some(addr.as_ref()) {
                // this record didn't originate from this agent
                continue;
            }
            let node_id = match location_record.subject {
                Subject::NymNode { node_id } => node_id,
            };
            let checked_at =
                OffsetDateTime::from_unix_timestamp(location_record.entry.checked_at as i64)?;

            let payload = location_record.entry.payload;

            let location = match payload.try_decode_v1() {
                Ok(location) => location,
                Err(err) => {
                    error!("failed to deserialize location for node {node_id}: {err}");
                    continue;
                }
            };

            nodes.insert(
                node_id,
                SubmittedLocation {
                    location,
                    checked_at,
                },
            );
        }

        Ok(OnChainNodes {
            nodes: Arc::new(RwLock::new(nodes)),
        })
    }

    pub(crate) async fn read(&self) -> RwLockReadGuard<'_, HashMap<NodeId, SubmittedLocation>> {
        self.nodes.read().await
    }

    pub(crate) async fn has_expired(&self, node_id: NodeId, ttl: Duration) -> bool {
        let guard = self.nodes.read().await;
        let Some(entry) = guard.get(&node_id) else {
            return true;
        };

        entry.has_expired(OffsetDateTime::now_utc(), ttl)
    }

    pub(crate) async fn update_submitted(&self, node: NodeId, location: Location) {
        // yes, it's not fully in async with chain, but that's perfectly fine, the only purpose
        // of this is for the task to know whether it should refresh the node ipinfo
        // (which happens on a multi-day cadence, so few seconds of desync are acceptable)
        self.nodes.write().await.insert(
            node,
            SubmittedLocation {
                location,
                checked_at: OffsetDateTime::now_utc(),
            },
        );
    }
}
