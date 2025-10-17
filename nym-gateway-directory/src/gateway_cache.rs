// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    net::IpAddr,
    time::{Duration, Instant},
};

use nym_offline_monitor::ConnectivityHandle;
use nym_sphinx::addressing::nodes::NodeIdentity;
use strum::IntoEnumIterator;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::{
    Error, Gateway, GatewayClient, GatewayFilters, GatewayList, GatewayType, error::Result,
};

/// The maximum age of the cache before it is considered stale.
const MAX_CACHE_AGE: Duration = Duration::from_secs(5 * 60);

#[derive(Clone)]
pub struct GatewayCacheHandle {
    tx: tokio::sync::mpsc::UnboundedSender<Command>,
}

impl GatewayCacheHandle {
    fn new(tx: tokio::sync::mpsc::UnboundedSender<Command>) -> Self {
        Self { tx }
    }

    /// Refresh all gateways and countries without blocking until the operation is complete.
    pub async fn refresh_all(&self) -> Result<()> {
        self.tx
            .send(Command::RefreshAll)
            .map_err(|_| Error::Cancelled)
    }

    /// Lookup gateways waiting for any pending fetch request or initiating one if needed.
    pub async fn lookup_gateways(&self, gw_type: GatewayType) -> Result<GatewayList> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(Command::LookupGateways(gw_type, tx))
            .map_err(|_| Error::Cancelled)?;
        rx.await.map_err(|_| Error::Cancelled)?
    }

    pub async fn lookup_filtered_gateways(&self, filters: GatewayFilters) -> Result<Vec<Gateway>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(Command::LookupFilteredGateways(filters, tx))
            .map_err(|_| Error::Cancelled)?;
        rx.await.map_err(|_| Error::Cancelled)?
    }

    /// Lookup gateway IP address waiting for any pending fetch request or initiating one if needed.
    pub async fn lookup_gateway_ip(&self, gateway_identity: String) -> Result<IpAddr> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(Command::LookupGatewayIp(gateway_identity, tx))
            .map_err(|_| Error::Cancelled)?;
        rx.await.map_err(|_| Error::Cancelled)?
    }

    pub fn replace_gateway_client(&mut self, gateway_client: GatewayClient) -> Result<()> {
        self.tx
            .send(Command::ReplaceGatewayClient(Box::new(gateway_client)))
            .map_err(|_| Error::Cancelled)
    }
}

enum Command {
    RefreshAll,
    LookupGateways(
        GatewayType,
        tokio::sync::oneshot::Sender<Result<GatewayList>>,
    ),
    LookupFilteredGateways(
        GatewayFilters,
        tokio::sync::oneshot::Sender<Result<Vec<Gateway>>>,
    ),
    LookupGatewayIp(
        String, // gateway_identity
        tokio::sync::oneshot::Sender<Result<IpAddr>>,
    ),
    ReplaceGatewayClient(Box<GatewayClient>),
}

pub struct GatewayCache {
    // The channel for receiving commands
    command_rx: tokio::sync::mpsc::UnboundedReceiver<Command>,

    // The underlying client that actually does the work
    gateway_client: GatewayClient,

    // The cached gateways and their last updated time
    cached_gateways: HashMap<GatewayType, (GatewayList, Instant)>,

    // The connectivity handle to check if we are online
    connectivity_handle: ConnectivityHandle,

    /// Whether the initial refresh has been performed
    is_performed_initial_refresh: bool,

    // Shutdown token
    shutdown_token: CancellationToken,
}

impl GatewayCache {
    pub fn spawn(
        gateway_client: GatewayClient,
        connectivity_handle: ConnectivityHandle,
        shutdown_token: CancellationToken,
    ) -> (GatewayCacheHandle, JoinHandle<()>) {
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();

        let inner = Self {
            gateway_client,
            connectivity_handle,
            command_rx,
            cached_gateways: HashMap::default(),
            is_performed_initial_refresh: false,
            shutdown_token,
        };
        let join_handle = tokio::spawn(inner.run());
        (GatewayCacheHandle::new(command_tx), join_handle)
    }

    async fn run(mut self) {
        if self.connectivity_handle.connectivity().await.is_online() {
            self.perform_initial_fetch_once().await;
        }

        loop {
            tokio::select! {
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        Command::RefreshAll => {
                            self.refresh_all().await;
                        }
                        Command::LookupGateways(gw_type, tx) => {
                            tx.send(self.lookup_gateways(gw_type).await).ok();
                        }
                        Command::LookupFilteredGateways(filters, tx) => {
                            let gw_vec = self.lookup_filtered_gateways(filters).await;
                            tx.send(gw_vec).ok();
                        }
                        Command::LookupGatewayIp(gateway_identity, tx) => {
                            tx.send(self.lookup_gateway_ip(&gateway_identity).await).ok();
                        }
                        Command::ReplaceGatewayClient(gateway_client) => {
                            self.replace_gateway_client(*gateway_client)
                        }
                    }
                }
                Some(status) = self.connectivity_handle.next() => {
                    if status.is_online() {
                        self.perform_initial_fetch_once().await;
                    }
                }
                _ = self.shutdown_token.cancelled() => {
                    break;
                }
            }
        }
    }

    async fn perform_initial_fetch_once(&mut self) {
        if !self.is_performed_initial_refresh {
            tracing::info!("Performing initial refresh");
            self.is_performed_initial_refresh = true;
            self.refresh_all().await;
        }
    }

    fn replace_gateway_client(&mut self, gateway_client: GatewayClient) {
        let old_config = self.gateway_client.get_config();
        let new_config = gateway_client.get_config();

        self.gateway_client = gateway_client;

        // Invalidate cache immediately if gateway performance or thresholds change
        if new_config.min_gateway_performance != old_config.min_gateway_performance
            || new_config.mix_score_thresholds != old_config.mix_score_thresholds
            || new_config.wg_score_thresholds != old_config.wg_score_thresholds
        {
            self.cached_gateways.clear();
        }
    }

    async fn refresh_all(&mut self) {
        let gw_types = self.get_stale_gateway_list_types();

        if !gw_types.is_empty() {
            tracing::info!("Refreshing gateways: {:?}", gw_types,);
            self.refresh(gw_types).await;
        }
    }

    fn get_stale_gateway_list_types(&self) -> Vec<GatewayType> {
        GatewayType::iter()
            .filter(|gw_type| !self.is_gateways_current(gw_type))
            .collect()
    }

    async fn refresh(&mut self, gw_list_types: Vec<GatewayType>) {
        if self.connectivity_handle.connectivity().await.is_offline() {
            tracing::debug!("Not refreshing gateways because we are not connected");
            return;
        }

        tracing::info!("Refreshing gateway lists: {gw_list_types:?}");

        let mut tasks = tokio::task::JoinSet::new();

        for gw_type in gw_list_types {
            let client = self.gateway_client.clone();
            tasks.spawn(async move {
                let res = client.lookup_gateways(gw_type).await;
                (gw_type, res)
            });
        }

        while let Some(res) = tasks.join_next().await {
            match res {
                Ok((gw_type, r)) => match r {
                    Ok(refreshed_gateways) => {
                        tracing::info!("Refreshed gateways for {gw_type:?}");
                        self.cached_gateways
                            .insert(gw_type, (refreshed_gateways, Instant::now()));
                    }
                    Err(err) => {
                        tracing::warn!("Failed to refresh gateways for {gw_type:?}: {err}");
                    }
                },
                Err(err) => {
                    tracing::error!("Failed to join on refresh task: {err}");
                }
            }
        }
    }

    fn is_gateways_current(&self, gw_type: &GatewayType) -> bool {
        self.cached_gateways
            .get(gw_type)
            .as_ref()
            .map(|(_, last_updated)| last_updated.elapsed() < MAX_CACHE_AGE)
            .unwrap_or_default()
    }

    async fn refresh_gateways(&mut self, gw_type: GatewayType) -> Result<GatewayList> {
        if let Some((gw_list, last_updated)) = self.cached_gateways.get(&gw_type)
            && last_updated.elapsed() < MAX_CACHE_AGE
        {
            Ok(gw_list.clone())
        } else {
            if self.connectivity_handle.connectivity().await.is_offline() {
                tracing::warn!("Not refreshing countries because we are not connected");
                return Err(Error::Offline);
            }

            let refreshed_gateways = self.gateway_client.lookup_gateways(gw_type).await?;

            self.cached_gateways
                .insert(gw_type, (refreshed_gateways.clone(), Instant::now()));

            Ok(refreshed_gateways)
        }
    }

    async fn lookup_gateways(&mut self, gw_type: GatewayType) -> Result<GatewayList> {
        let refresh_result = self.refresh_gateways(gw_type).await;

        // Regardless of if we managed to refresh the cache, we return the cached gateways if they
        // exist. They should be the most recent one we can muster
        if let Some((gateways, _)) = self.cached_gateways.get(&gw_type) {
            Ok(gateways.clone())
        } else {
            refresh_result
        }
    }

    async fn lookup_filtered_gateways(&mut self, filters: GatewayFilters) -> Result<Vec<Gateway>> {
        let gw_list = self.lookup_gateways(filters.gw_type).await?;
        Ok(gw_list.filter(&filters.filters))
    }

    async fn lookup_gateway_ip(&mut self, gateway_identity: &str) -> Result<IpAddr> {
        // If we have a populated list of gateways, we should always be able to find the IP there.
        if let Ok(identity) = NodeIdentity::from_base58_string(gateway_identity) {
            for (_, (gateways, _)) in self.cached_gateways.iter() {
                if let Some(ip) = gateways
                    .node_with_identity(&identity)
                    .and_then(Gateway::lookup_ip)
                {
                    return Ok(ip);
                }
            }
        } else {
            tracing::warn!("Failed to parse gateway identity: {gateway_identity}");
        }

        // Fallback
        tracing::warn!("Using fallback to lookup gateway IP");
        self.gateway_client
            .lookup_gateway_ip(gateway_identity)
            .await
    }
}
