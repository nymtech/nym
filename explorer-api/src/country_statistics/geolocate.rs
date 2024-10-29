// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::state::ExplorerApiStateContext;
use log::{info, warn};
use nym_explorer_api_requests::Location;
use nym_network_defaults::DEFAULT_NYM_NODE_HTTP_PORT;
use nym_task::TaskClient;

pub(crate) struct GeoLocateTask {
    state: ExplorerApiStateContext,
    shutdown: TaskClient,
}

impl GeoLocateTask {
    pub(crate) fn new(state: ExplorerApiStateContext, shutdown: TaskClient) -> Self {
        GeoLocateTask { state, shutdown }
    }

    pub(crate) fn start(mut self) {
        info!("Spawning locator task runner...");
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(std::time::Duration::from_millis(50));
            while !self.shutdown.is_shutdown() {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        self.locate_mix_nodes().await;
                        self.locate_gateways().await;
                        self.locate_nym_nodes().await;
                    }
                    _ = self.shutdown.recv() => {
                        trace!("Listener: Received shutdown");
                    }
                }
            }
        });
    }

    async fn locate_mix_nodes(&mut self) {
        // I'm unwrapping to the default value to get rid of an extra indentation level from the `if let Some(...) = ...`
        // If the value is None, we'll unwrap to an empty hashmap and the `values()` loop won't do any work anyway
        let mixnode_bonds = self
            .state
            .inner
            .mixnodes
            .get_mixnodes()
            .await
            .unwrap_or_default();

        let geo_ip = self.state.inner.geo_ip.0.clone();

        for (i, cache_item) in mixnode_bonds.values().enumerate() {
            if self
                .state
                .inner
                .mixnodes
                .is_location_valid(cache_item.mix_id())
                .await
            {
                // when the cached location is valid, don't locate and continue to next mix node
                continue;
            }

            match geo_ip.query(
                &cache_item.mix_node().host,
                Some(cache_item.mix_node().mix_port),
            ) {
                Ok(opt) => match opt {
                    Some(location) => {
                        let location: Location = location.into();

                        trace!(
                            "{} mix nodes already located. Ip {} is located in {:#?}",
                            i,
                            cache_item.mix_node().host,
                            location.three_letter_iso_country_code,
                        );

                        if i > 0 && (i % 100) == 0 {
                            info!("Located {} mixnodes...", i + 1,);
                        }

                        self.state
                            .inner
                            .mixnodes
                            .set_location(cache_item.mix_id(), Some(location))
                            .await;

                        // one node has been located, so return out of the loop
                        return;
                    }
                    None => {
                        warn!("❌ Location for {} not found.", cache_item.mix_node().host);
                        self.state
                            .inner
                            .mixnodes
                            .set_location(cache_item.mix_id(), None)
                            .await;
                    }
                },
                Err(_e) => {
                    // warn!(
                    //     "❌ Oh no! Location for {} failed. Error: {:#?}",
                    //     cache_item.mix_node().host,
                    //     e
                    // );
                }
            };
        }

        trace!("All mix nodes located");
    }

    async fn locate_nym_nodes(&mut self) {
        // I'm unwrapping to the default value to get rid of an extra indentation level from the `if let Some(...) = ...`
        // If the value is None, we'll unwrap to an empty hashmap and the `values()` loop won't do any work anyway
        let nym_nodes = self.state.inner.nymnodes.get_bonded_nymnodes().await;

        let geo_ip = self.state.inner.geo_ip.0.clone();

        for (i, cache_item) in nym_nodes.values().enumerate() {
            if self
                .state
                .inner
                .nymnodes
                .is_location_valid(cache_item.node_id())
                .await
            {
                // when the cached location is valid, don't locate and continue to next mix node
                continue;
            }

            let bonded_host = &cache_item.bond_information.node.host;

            match geo_ip.query(
                bonded_host,
                Some(
                    cache_item
                        .bond_information
                        .node
                        .custom_http_port
                        .unwrap_or(DEFAULT_NYM_NODE_HTTP_PORT),
                ),
            ) {
                Ok(opt) => match opt {
                    Some(location) => {
                        let location: Location = location.into();

                        trace!(
                            "{} mix nodes already located. host {} is located in {:#?}",
                            i,
                            bonded_host,
                            location.three_letter_iso_country_code,
                        );

                        if i > 0 && (i % 100) == 0 {
                            info!("Located {} nym-nodes...", i + 1,);
                        }

                        self.state
                            .inner
                            .nymnodes
                            .set_location(cache_item.node_id(), Some(location))
                            .await;

                        // one node has been located, so return out of the loop
                        return;
                    }
                    None => {
                        warn!("❌ Location for {bonded_host} not found.");
                        self.state
                            .inner
                            .nymnodes
                            .set_location(cache_item.node_id(), None)
                            .await;
                    }
                },
                Err(_e) => {
                    // warn!(
                    //     "❌ Oh no! Location for {} failed. Error: {:#?}",
                    //     cache_item.mix_node().host,
                    //     e
                    // );
                }
            };
        }

        trace!("All nym-nodes nodes located");
    }

    async fn locate_gateways(&mut self) {
        let gateways = self.state.inner.gateways.get_gateways().await;

        let geo_ip = self.state.inner.geo_ip.0.clone();

        for (i, cache_item) in gateways.iter().enumerate() {
            if self
                .state
                .inner
                .gateways
                .is_location_valid(cache_item.identity().to_owned())
                .await
            {
                // when the cached location is valid, don't locate and continue to next gateway
                continue;
            }

            match geo_ip.query(&cache_item.gateway.host, Some(cache_item.gateway.mix_port)) {
                Ok(opt) => match opt {
                    Some(location) => {
                        let location: Location = location.into();

                        trace!(
                            "{} gateways already located. Ip {} is located in {:#?}",
                            i,
                            cache_item.gateway.host,
                            location.three_letter_iso_country_code,
                        );

                        if i > 0 && (i % 100) == 0 {
                            info!("Located {} gateways...", i + 1,);
                        }

                        self.state
                            .inner
                            .gateways
                            .set_location(cache_item.identity().to_owned(), Some(location))
                            .await;

                        // one node has been located, so return out of the loop
                        return;
                    }
                    None => {
                        warn!("❌ Location for {} not found.", cache_item.gateway.host);
                        self.state
                            .inner
                            .gateways
                            .set_location(cache_item.identity().to_owned(), None)
                            .await;
                    }
                },
                Err(_e) => {
                    // warn!(
                    //     "❌ Oh no! Location for {} failed. Error: {:#?}",
                    //     cache_item.gateway.host,
                    //     e
                    // );
                }
            };
        }

        trace!("All gateways located");
    }
}
