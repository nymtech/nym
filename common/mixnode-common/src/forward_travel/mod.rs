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

pub type AllowedIngress = AllowedPaths;
pub type AllowedEgress = AllowedPaths;

pub struct AllowedAddressesProvider {
    current_epoch: EpochId,

    identity: IdentityKey,

    client_config: nyxd::Config,

    /// URLs to the nyxd validators for obtaining unfiltered network topology.
    nyxd_endpoints: Vec<Url>,

    ingress: AllowedIngress,
    egress: AllowedEgress,
}

impl AllowedAddressesProvider {
    pub fn new(
        identity: IdentityKey,
        nyxd_endpoints: Vec<Url>,
        network_details: Option<NymNetworkDetails>,
    ) -> Result<Self, ForwardTravelError> {
        let network = network_details.unwrap_or(NymNetworkDetails::new_mainnet());
        Ok(AllowedAddressesProvider {
            current_epoch: 0,
            identity,
            client_config: nyxd::Config::try_from_nym_network_details(&network)?,
            nyxd_endpoints,
            ingress: AllowedPaths::new(),
            egress: AllowedPaths::new(),
        })
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

    pub fn ingress(&self) -> AllowedIngress {
        AllowedIngress {
            inner: Arc::clone(&self.ingress.inner),
        }
    }

    pub fn egress(&self) -> AllowedEgress {
        AllowedEgress {
            inner: Arc::clone(&self.egress.inner),
        }
    }

    fn add_node_ips(raw_host: &str, identity: &str, set: &mut HashSet<IpAddr>) {
        if let Ok(ip) = IpAddr::from_str(raw_host) {
            set.insert(ip);
        } else {
            // this might still be a valid hostname
            //
            // annoyingly there exists a method of looking up a socket address but not an ip address,
            // so append any port and perform the lookup
            let Ok(sockets) = format!("{raw_host}:1789").to_socket_addrs() else {
                warn!("failed to resolve ip address of node '{identity}' (hostname: {raw_host})");
                return;
            };

            for socket in sockets {
                set.insert(socket.ip());
            }
        }
    }

    fn get_all_addresses(nodes: &[MixNodeBond], gateways: &[GatewayBond]) -> HashSet<IpAddr> {
        let mut allowed = HashSet::new();

        for node in nodes.iter() {
            Self::add_node_ips(&node.mix_node.host, node.identity(), &mut allowed);
        }

        for gateway in gateways.iter() {
            Self::add_node_ips(&gateway.gateway.host, gateway.identity(), &mut allowed);
        }

        allowed
    }

    /// Gets ip addresses of all mixnodes on given layer
    fn get_addresses_on_layer(layer: Layer, nodes: &[MixNodeBond]) -> HashSet<IpAddr> {
        let mut allowed = HashSet::new();

        for node in nodes.iter().filter(|m| m.layer == layer) {
            Self::add_node_ips(&node.mix_node.host, node.identity(), &mut allowed);
        }

        allowed
    }

    fn gateway_addresses(gateways: &[GatewayBond]) -> HashSet<IpAddr> {
        let mut allowed = HashSet::new();

        for gateway in gateways.iter() {
            Self::add_node_ips(&gateway.gateway.host, gateway.identity(), &mut allowed);
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
            // layer 1
            (None, Some(next)) => {
                let gateways = client.get_all_gateways().await?;

                (
                    Self::gateway_addresses(&gateways),
                    Self::get_addresses_on_layer(next, &mixnodes),
                )
            }
            // layer 2
            (Some(previous), Some(next)) => (
                Self::get_addresses_on_layer(previous, &mixnodes),
                Self::get_addresses_on_layer(next, &mixnodes),
            ),
            // layer 3
            (Some(previous), None) => {
                let gateways = client.get_all_gateways().await?;
                (
                    Self::get_addresses_on_layer(previous, &mixnodes),
                    Self::gateway_addresses(&gateways),
                )
            }
            // gateway (or not bonded)
            (None, None) => {
                let gateways = client.get_all_gateways().await?;

                if self.is_gateway(&gateways) {
                    let mut base_ingress = Self::get_addresses_on_layer(Layer::Three, &mixnodes);
                    let mut base_egress = Self::get_addresses_on_layer(Layer::One, &mixnodes);

                    // TODO: this extension should be conditional on whether the node is running the vpn module
                    let gw_extension = Self::gateway_addresses(&gateways);

                    base_ingress.extend(gw_extension.clone());
                    base_egress.extend(gw_extension.clone());

                    (base_ingress, base_egress)
                } else {
                    warn!("our node doesn't appear to be bonded - going to permit traffic from ALL mixnodes and gateways");
                    let all = Self::get_all_addresses(&mixnodes, &gateways);
                    (all.clone(), all)
                }
            }
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

    pub async fn run(&mut self, mut task_client: TaskClient) {
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
