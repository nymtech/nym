// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::forward_travel::error::ForwardTravelError;
use log::{debug, error, trace, warn};
use nym_mixnet_contract_common::{EpochId, GatewayBond, Layer, MixNodeBond};
use nym_network_defaults::NymNetworkDetails;
use nym_task::TaskClient;
use nym_validator_client::client::IdentityKey;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, PagedMixnetQueryClient};
use nym_validator_client::{nyxd, QueryHttpRpcNyxdClient};
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::collections::HashSet;
use std::mem;
use std::net::{IpAddr, ToSocketAddrs};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::{sync::RwLock, time::sleep};
use url::Url;

pub mod error;

pub type Ingress = AllowedPaths;
pub type Egress = AllowedPaths;

pub struct AllowedAddressesProvider {
    current_epoch: EpochId,

    identity: IdentityKey,

    client_config: nyxd::Config,

    /// URLs to the nyxd validators for obtaining unfiltered network topology.
    nyxd_endpoints: Vec<Url>,

    ingress: Ingress,
    egress: Egress,
}

impl AllowedAddressesProvider {
    fn new(&self, network_details: Option<NymNetworkDetails>) -> Result<Self, ForwardTravelError> {
        todo!()
    }

    fn ephemeral_nyxd_client(&self) -> Result<QueryHttpRpcNyxdClient, ForwardTravelError> {
        let mut possible_nyxd_endpoints = self.nyxd_endpoints.clone();
        possible_nyxd_endpoints.shuffle(&mut thread_rng());

        let mut last_error = match QueryHttpRpcNyxdClient::connect(
            self.client_config.clone(),
            possible_nyxd_endpoints
                .pop()
                .ok_or(ForwardTravelError::NoNyxdUrlsAvailable)?
                .as_str(),
        ) {
            Ok(client) => return Ok(client),
            Err(err) => err,
        };

        for url in possible_nyxd_endpoints {
            match QueryHttpRpcNyxdClient::connect(self.client_config.clone(), url.as_str()) {
                Ok(client) => return Ok(client),
                Err(err) => last_error = err,
            };
        }

        Err(last_error.into())
    }

    pub fn ingress(&self) -> Ingress {
        Ingress {
            inner: Arc::clone(&self.ingress.inner),
        }
    }

    pub fn egress(&self) -> Egress {
        Egress {
            inner: Arc::clone(&self.egress.inner),
        }
    }

    /// Gets ip addresses of all mixnodes on given layer
    fn get_addresses_on_layer(layer: Layer, nodes: &[MixNodeBond]) -> HashSet<IpAddr> {
        let mut allowed = HashSet::new();

        for node in nodes.iter().filter(|m| m.layer == layer) {
            match IpAddr::from_str(&node.mix_node.host) {
                Ok(ip) => {
                    allowed.insert(ip);
                }
                Err(_) => {
                    // this might still be a valid hostname

                    // annoyingly there exists a method of looking up a socket address but not an ip address,
                    // so append any port and perform the lookup
                    let Ok(sockets) = format!("{}:{}", node.mix_node.host, node.mix_node.mix_port)
                        .to_socket_addrs()
                    else {
                        warn!(
                            "failed to resolve ip address of mixnode '{}' (hostname: {})",
                            node.identity(),
                            node.mix_node.host
                        );
                        continue;
                    };

                    for socket in sockets {
                        allowed.insert(socket.ip());
                    }
                }
            }
        }

        allowed
    }

    fn gateway_addresses(gateways: &[GatewayBond]) -> HashSet<IpAddr> {
        let mut allowed = HashSet::new();

        for gateway in gateways.iter() {
            match IpAddr::from_str(&gateway.gateway.host) {
                Ok(ip) => {
                    allowed.insert(ip);
                }
                Err(_) => {
                    // this might still be a valid hostname

                    // annoyingly there exists a method of looking up a socket address but not an ip address,
                    // so append any port and perform the lookup
                    let Ok(sockets) =
                        format!("{}:{}", gateway.gateway.host, gateway.gateway.mix_port)
                            .to_socket_addrs()
                    else {
                        warn!(
                            "failed to resolve ip address of gateway '{}' (hostname: {})",
                            gateway.identity(),
                            gateway.gateway.host
                        );
                        continue;
                    };

                    for socket in sockets {
                        allowed.insert(socket.ip());
                    }
                }
            }
        }

        allowed
    }

    fn locate_layer(&self, nodes: &[MixNodeBond]) -> Option<Layer> {
        nodes
            .iter()
            .find(|m| m.identity() == self.identity)
            .map(|m| m.layer)
    }

    fn is_gateway(&self, gateways: &[GatewayBond]) -> bool {
        gateways
            .iter()
            .find(|g| g.gateway.identity_key == self.identity)
            .is_some()
    }

    async fn update_state(
        &mut self,
        client: QueryHttpRpcNyxdClient,
    ) -> Result<(), ForwardTravelError> {
        let current_interval = client.get_current_interval_details().await?;
        let current_epoch = current_interval.interval.current_epoch_absolute_id();

        if current_epoch == self.current_epoch {
            error!("can't update the allowed ips list as the epoch appears to be stuck");
            return Err(ForwardTravelError::StuckEpoch);
        }

        let has_epoch_deviated = current_epoch > self.current_epoch + 1;

        let mixnodes = client.get_all_mixnode_bonds().await?;
        let our_mix_layer = self.locate_layer(&mixnodes);

        let previous_mix_layer = our_mix_layer.map(|l| l.try_previous()).flatten();
        let next_mix_layer = our_mix_layer.map(|l| l.try_next()).flatten();

        let (allowed_ingress, allowed_egress) = match (previous_mix_layer, next_mix_layer) {
            (Some(previous), Some(next)) => (
                Self::get_addresses_on_layer(previous, &mixnodes),
                Self::get_addresses_on_layer(next, &mixnodes),
            ),
            (Some(previous), None) => {
                let gateways = client.get_all_gateways().await?;
                (
                    Self::get_addresses_on_layer(previous, &mixnodes),
                    Self::gateway_addresses(&gateways),
                )
            }
            (None, Some(next)) => {
                let gateways = client.get_all_gateways().await?;

                (
                    Self::gateway_addresses(&gateways),
                    Self::get_addresses_on_layer(next, &mixnodes),
                )
            }

            _ => unreachable!("both previous and next layers are set to be gateways!"),
        };

        self.current_epoch = current_epoch;
        self.ingress
            .advance_epoch(allowed_ingress, has_epoch_deviated)
            .await;
        self.egress
            .advance_epoch(allowed_egress, has_epoch_deviated)
            .await;

        Ok(())
    }

    async fn update_allowed_addresses(&mut self) -> Result<(), ForwardTravelError> {
        // create new client every epoch because it results in different nyxd endpoint being used
        // what may help in distributing the load
        let client = self.ephemeral_nyxd_client()?;
        self.wait_for_epoch_rollover(&client).await?;
        self.update_state(client).await
    }

    async fn wait_for_epoch_rollover(
        &self,
        client: &QueryHttpRpcNyxdClient,
    ) -> Result<(), ForwardTravelError> {
        let current_interval = client.get_current_interval_details().await?;
        let current_epoch = current_interval.interval.current_epoch_absolute_id();

        if current_epoch <= self.current_epoch {
            let remaining = current_interval.time_until_current_epoch_end();
            // add few more seconds to account for block time drift and to spread queries of all
            // other nodes
            let adjustment_secs = rand::thread_rng().gen_range(5..90);
            sleep(remaining + Duration::from_secs(adjustment_secs)).await;
        }

        Ok(())
    }

    async fn run(&mut self, mut task_client: TaskClient) {
        debug!("Started ValidAddressesProvider with graceful shutdown support");
        while !task_client.is_shutdown() {
            tokio::select! {
                biased;
                _ = task_client.recv() => {
                    trace!("ValidAddressesProvider: Received shutdown");
                }
                res = self.update_allowed_addresses() => {
                    if let Err(err) = res {
                        warn!("failed to update the allowed addresses: {err}");
                        // don't retry immediately in case it was a network failure, wait a bit instead.
                        tokio::select! {
                            biased;
                            _ = task_client.recv() => {
                                trace!("ValidAddressesProvider: Received shutdown");
                            }
                            _ = sleep(Duration::from_secs(5 * 60)) => {}
                        }
                    }
                }
            }
        }
        task_client.recv_timeout().await;
        log::debug!("ValidAddressesProvider: Exiting");
    }
}

pub struct AllowedPaths {
    inner: Arc<RwLock<AllowedPathsInner>>,
}

impl AllowedPaths {
    fn new() -> Self {
        AllowedPaths {
            inner: Arc::new(RwLock::new(AllowedPathsInner {
                previous_epoch: HashSet::new(),
                current_epoch: HashSet::new(),
            })),
        }
    }

    pub async fn is_allowed(&self, address: IpAddr) -> bool {
        let guard = self.inner.read().await;
        guard.current_epoch.contains(&address) || guard.previous_epoch.contains(&address)
    }

    async fn advance_epoch(&self, current_epoch: HashSet<IpAddr>, reset_previous: bool) {
        let mut guard = self.inner.write().await;

        let old_current = mem::replace(&mut guard.current_epoch, current_epoch);

        if reset_previous {
            guard.previous_epoch = HashSet::new()
        } else {
            guard.previous_epoch = old_current;
        }
    }
}

struct AllowedPathsInner {
    previous_epoch: HashSet<IpAddr>,
    current_epoch: HashSet<IpAddr>,
}
