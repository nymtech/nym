// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nyx::client::NyxClient;
use crate::nyx::state::OnChainNodes;
use nym_geolocation_contract_common::payload::Location;
use nym_geolocation_contract_common::{LocationPayload, Measurement, Method, Subject};
use nym_validator_client::nyxd::contract_traits::GeolocationSigningClient;
use nym_validator_client::nyxd::nym_performance_contract_common::NodeId;
use tracing::info;

#[derive(Clone)]
pub(crate) struct LocationPusher {
    batch_size: usize,
    client: NyxClient,
    on_chain_cache: OnChainNodes,
}

impl LocationPusher {
    pub(crate) fn new(client: NyxClient, on_chain_cache: OnChainNodes, batch_size: usize) -> Self {
        Self {
            batch_size,
            client,
            on_chain_cache,
        }
    }

    pub(crate) async fn push_updates(
        &self,
        updates: Vec<(NodeId, Location)>,
    ) -> anyhow::Result<()> {
        info!("attempting to submit {} location updates", updates.len());
        let batches = updates.chunks(self.batch_size);
        let batch_len = batches.size_hint().0;

        for (i, batch) in batches.enumerate() {
            info!("pushing batch {}/{}", i + 1, batch_len);

            let mut measurements = Vec::with_capacity(batch.len());
            for (node_id, location) in batch {
                measurements.push(Measurement {
                    subject: Subject::new_nym_node(*node_id),
                    method: Method::IpInfo,
                    payload: LocationPayload::new_v1(location)?,
                });
            }

            // 1. push data to chain
            self.client.submit_measurements(measurements, None).await?;

            // 2. update local cache
            self.on_chain_cache.update_submitted(batch.to_vec()).await;
        }

        Ok(())
    }
}
