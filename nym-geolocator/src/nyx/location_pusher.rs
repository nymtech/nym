// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nyx::client::NyxClient;
use crate::nyx::state::OnChainNodes;
use anyhow::bail;
use nym_geolocation_contract_common::payload::Location;
use nym_geolocation_contract_common::{
    ContractConfig, LocationPayload, Measurement, Method, Subject,
};
use nym_validator_client::nyxd::contract_traits::GeolocationSigningClient;
use nym_validator_client::nyxd::nym_performance_contract_common::NodeId;
use tracing::{error, info, warn};

#[derive(Clone)]
pub(crate) struct LocationPusher {
    batch_size: usize,
    max_payload_size: u32,
    client: NyxClient,
    on_chain_cache: OnChainNodes,
}

impl LocationPusher {
    pub(crate) fn new(
        client: NyxClient,
        on_chain_cache: OnChainNodes,
        contract_config: &ContractConfig,
    ) -> Self {
        Self {
            batch_size: contract_config.max_batch_size as usize,
            max_payload_size: contract_config.max_payload_size,
            client,
            on_chain_cache,
        }
    }

    /// Encode one measurement, rejecting what the contract would reject.
    ///
    /// Batches are all-or-nothing, so an entry the contract refuses does not merely fail itself:
    /// it takes every other measurement in its batch down with it. Dropping it here costs one
    /// node its entry for this cycle instead, and it will be retried on the next.
    fn encode(&self, node_id: NodeId, location: &Location) -> anyhow::Result<Measurement> {
        let payload = LocationPayload::new_v1(location)?;
        payload.ensure_within_size_limit(self.max_payload_size)?;

        Ok(Measurement {
            subject: Subject::new_nym_node(node_id),
            method: Method::IpInfo,
            payload,
        })
    }

    pub(crate) async fn push_updates(
        &self,
        updates: Vec<(NodeId, Location)>,
    ) -> anyhow::Result<()> {
        info!("attempting to submit {} location updates", updates.len());
        let batches = updates.chunks(self.batch_size);
        let batch_len = batches.size_hint().0;

        let mut failed_batches = 0;
        for (i, batch) in batches.enumerate() {
            info!("pushing batch {}/{}", i + 1, batch_len);

            let mut measurements = Vec::with_capacity(batch.len());
            let mut submitted = Vec::with_capacity(batch.len());
            for (node_id, location) in batch {
                match self.encode(*node_id, location) {
                    Ok(measurement) => {
                        measurements.push(measurement);
                        submitted.push((*node_id, location.clone()));
                    }
                    Err(err) => {
                        warn!("dropping the measurement of node {node_id}: {err}");
                    }
                }
            }

            if measurements.is_empty() {
                continue;
            }

            // 1. push data to chain. each batch is its own transaction, so one that fails - a
            // chain hiccup, or an entry that got past the checks above - must not take the
            // batches after it with it, which `?` here would do
            if let Err(err) = self.client.submit_measurements(measurements, None).await {
                error!("failed to submit batch {}/{}: {err}", i + 1, batch_len);
                failed_batches += 1;
                continue;
            }

            // 2. update local cache
            self.on_chain_cache.update_submitted(submitted).await;
        }

        if failed_batches > 0 {
            bail!("{failed_batches} out of {batch_len} batch(es) failed to submit")
        }

        Ok(())
    }
}
