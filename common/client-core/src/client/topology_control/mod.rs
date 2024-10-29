// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::spawn_future;
pub(crate) use accessor::{TopologyAccessor, TopologyReadPermit};
use futures::StreamExt;
use log::*;
use nym_sphinx::addressing::nodes::NodeIdentity;
use nym_topology::NymTopologyError;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::sleep;

#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::sleep;

mod accessor;
pub mod geo_aware_provider;
pub mod nym_api_provider;

pub use geo_aware_provider::GeoAwareTopologyProvider;
pub use nym_api_provider::{Config as NymApiTopologyProviderConfig, NymApiTopologyProvider};
pub use nym_topology::provider_trait::TopologyProvider;

// TODO: move it to config later
const MAX_FAILURE_COUNT: usize = 10;

pub struct TopologyRefresherConfig {
    refresh_rate: Duration,
}

impl TopologyRefresherConfig {
    pub fn new(refresh_rate: Duration) -> Self {
        TopologyRefresherConfig { refresh_rate }
    }
}

pub struct TopologyRefresher {
    topology_provider: Box<dyn TopologyProvider + Send + Sync>,
    topology_accessor: TopologyAccessor,

    refresh_rate: Duration,
    consecutive_failure_count: usize,
}

impl TopologyRefresher {
    pub fn new(
        cfg: TopologyRefresherConfig,
        topology_accessor: TopologyAccessor,
        topology_provider: Box<dyn TopologyProvider + Send + Sync>,
    ) -> Self {
        TopologyRefresher {
            topology_provider,
            topology_accessor,
            refresh_rate: cfg.refresh_rate,
            consecutive_failure_count: 0,
        }
    }

    pub fn change_topology_provider(&mut self, provider: Box<dyn TopologyProvider + Send + Sync>) {
        self.topology_provider = provider;
    }

    pub async fn try_refresh(&mut self) {
        trace!("Refreshing the topology");

        if self.topology_accessor.controlled_manually() {
            info!("topology is being controlled manually - we're going to wait until the control is released...");
            self.topology_accessor
                .wait_for_released_manual_control()
                .await;
        }

        let new_topology = self.topology_provider.get_new_topology().await;
        if new_topology.is_none() {
            warn!("failed to obtain new network topology");
        }

        if new_topology.is_none() && self.consecutive_failure_count < MAX_FAILURE_COUNT {
            // if we failed to grab this topology, but the one before it was alright, let's assume
            // validator had a tiny hiccup and use the old data
            warn!("we're going to keep on using the old topology for this iteration");
            self.consecutive_failure_count += 1;
            return;
        } else if new_topology.is_some() {
            self.consecutive_failure_count = 0;
        }

        self.topology_accessor
            .update_global_topology(new_topology)
            .await;
    }

    pub async fn ensure_topology_is_routable(&self) -> Result<(), NymTopologyError> {
        self.topology_accessor.ensure_is_routable().await
    }

    pub async fn ensure_contains_gateway(
        &self,
        gateway: &NodeIdentity,
    ) -> Result<(), NymTopologyError> {
        let topology = self
            .topology_accessor
            .current_topology()
            .await
            .ok_or(NymTopologyError::EmptyNetworkTopology)?;

        if !topology.gateway_exists(gateway) {
            return Err(NymTopologyError::NonExistentGatewayError {
                identity_key: gateway.to_base58_string(),
            });
        }

        Ok(())
    }

    pub async fn wait_for_gateway(
        &mut self,
        gateway: &NodeIdentity,
        timeout_duration: Duration,
    ) -> Result<(), NymTopologyError> {
        info!(
            "going to wait for at most {timeout_duration:?} for gateway '{gateway}' to come online"
        );

        let deadline = sleep(timeout_duration);
        tokio::pin!(deadline);

        loop {
            tokio::select! {
                _ = &mut deadline => {
                    return Err(NymTopologyError::TimedOutWaitingForGateway {
                        identity_key: gateway.to_base58_string()
                    })
                }
                _ = self.try_refresh() => {
                    if self.ensure_contains_gateway(gateway).await.is_ok() {
                        return Ok(())
                    }
                    info!("gateway '{gateway}' is still not online...");
                    sleep(self.refresh_rate).await
                }
            }
        }
    }

    pub fn start_with_shutdown(mut self, mut shutdown: nym_task::TaskClient) {
        spawn_future(async move {
            debug!("Started TopologyRefresher with graceful shutdown support");

            #[cfg(not(target_arch = "wasm32"))]
            let mut interval = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
                self.refresh_rate,
            ));

            #[cfg(target_arch = "wasm32")]
            let mut interval =
                gloo_timers::future::IntervalStream::new(self.refresh_rate.as_millis() as u32);

            while !shutdown.is_shutdown() {
                tokio::select! {
                    _ = interval.next() => {
                        self.try_refresh().await;
                    },
                    _ = shutdown.recv() => {
                        log::trace!("TopologyRefresher: Received shutdown");
                    },
                }
            }
            shutdown.recv_timeout().await;
            log::debug!("TopologyRefresher: Exiting");
        })
    }
}
