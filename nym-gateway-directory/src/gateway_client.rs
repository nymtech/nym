// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    fmt,
    net::{IpAddr, SocketAddr},
};

use crate::{
    Error, GatewayMinPerformance, ScoreThresholds,
    entries::gateway::{Gateway, GatewayList, GatewayType, NymNodeList},
    error::Result,
};
use nym_contracts_common::Percent;
use nym_http_api_client::UserAgent;
use nym_validator_client::{
    models::NymNodeDescription, nym_api::NymApiClientExt, nym_nodes::SkimmedNodesWithMetadata,
};
use rand::{prelude::SliceRandom, thread_rng};
use tracing::{debug, error, warn};
use url::Url;

#[derive(Clone, Debug)]
pub struct Config {
    pub nyxd_url: Url,
    pub api_url: Url,
    pub min_gateway_performance: Option<GatewayMinPerformance>,
    pub mix_score_thresholds: Option<ScoreThresholds>,
    pub wg_score_thresholds: Option<ScoreThresholds>,
}

fn to_string<T: fmt::Display>(value: &Option<T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "unset".to_string(),
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "nyxd_url: {}, api_url: {}", self.nyxd_url, self.api_url,)
    }
}

impl Config {
    pub fn nyxd_url(&self) -> &Url {
        &self.nyxd_url
    }

    pub fn with_custom_nyxd_url(mut self, nyxd_url: Url) -> Self {
        self.nyxd_url = nyxd_url;
        self
    }

    pub fn api_url(&self) -> &Url {
        &self.api_url
    }

    pub fn with_custom_api_url(mut self, api_url: Url) -> Self {
        self.api_url = api_url;
        self
    }

    pub fn with_min_gateway_performance(
        mut self,
        min_gateway_performance: GatewayMinPerformance,
    ) -> Self {
        self.min_gateway_performance = Some(min_gateway_performance);
        self
    }
}

#[derive(Clone)]
pub struct GatewayClient {
    api_client: nym_http_api_client::Client,
    nyxd_url: Url,
    min_gateway_performance: Option<GatewayMinPerformance>,
    mix_score_thresholds: Option<ScoreThresholds>,
    wg_score_thresholds: Option<ScoreThresholds>,
}

impl GatewayClient {
    pub fn new(config: Config, user_agent: UserAgent) -> Result<Self> {
        Self::new_with_resolver_overrides(config, user_agent, None)
    }

    pub fn new_with_resolver_overrides(
        config: Config,
        user_agent: UserAgent,
        _static_nym_api_ip_addresses: Option<&[SocketAddr]>,
    ) -> Result<Self> {
        let api_client = nym_http_api_client::Client::builder(config.api_url.clone())
            .map_err(|e| Error::FailedToLookupDescribedGateways(e.into()))?
            .with_user_agent(user_agent.clone())
            .build()
            .map_err(|e| Error::FailedToLookupDescribedGateways(e.into()))?;

        Ok(GatewayClient {
            api_client,
            nyxd_url: config.nyxd_url,
            min_gateway_performance: config.min_gateway_performance,
            mix_score_thresholds: config.mix_score_thresholds,
            wg_score_thresholds: config.wg_score_thresholds,
        })
    }

    pub fn from_network_with_resolver_overrides(
        config: Config,
        network_details: &nym_network_defaults::NymNetworkDetails,
        user_agent: UserAgent,
        _static_nym_api_ip_addresses: Option<&[SocketAddr]>,
    ) -> Result<Self> {
        let api_client = nym_http_api_client::ClientBuilder::from_network(network_details)
            .map_err(Box::new)?
            .with_user_agent(user_agent.clone())
            .build()
            .map_err(Box::new)?;

        Ok(GatewayClient {
            api_client,
            nyxd_url: config.nyxd_url,
            min_gateway_performance: config.min_gateway_performance,
            mix_score_thresholds: config.mix_score_thresholds,
            wg_score_thresholds: config.wg_score_thresholds,
        })
    }

    /// Return the config of this instance.
    pub fn get_config(&self) -> Config {
        Config {
            api_url: self.api_client.api_url().clone(),
            nyxd_url: self.nyxd_url.clone(),
            min_gateway_performance: self.min_gateway_performance,
            mix_score_thresholds: self.mix_score_thresholds,
            wg_score_thresholds: self.wg_score_thresholds,
        }
    }

    pub fn mixnet_min_performance(&self) -> Option<Percent> {
        self.min_gateway_performance
            .as_ref()
            .and_then(|min_performance| min_performance.mixnet_min_performance)
    }

    pub fn vpn_min_performance(&self) -> Option<Percent> {
        self.min_gateway_performance
            .as_ref()
            .and_then(|min_performance| min_performance.vpn_min_performance)
    }

    async fn lookup_described_nodes(&self) -> Result<Vec<NymNodeDescription>> {
        debug!("Fetching all described nodes from nym-api...");
        self.api_client
            .get_all_described_nodes()
            .await
            .map_err(|e| Error::NymApi {
                source: Box::new(e),
            })
    }

    async fn lookup_skimmed_gateways(&self) -> Result<SkimmedNodesWithMetadata> {
        debug!("Fetching skimmed entry assigned nodes from nym-api...");
        self.api_client
            .get_all_basic_entry_assigned_nodes_with_metadata()
            .await
            .map_err(|e| Error::NymApi {
                source: Box::new(e),
            })
    }

    async fn lookup_skimmed_nodes(&self) -> Result<SkimmedNodesWithMetadata> {
        debug!("Fetching skimmed entry assigned nodes from nym-api...");
        self.api_client
            .get_all_basic_nodes_with_metadata()
            .await
            .map_err(|e| Error::NymApi {
                source: Box::new(e),
            })
    }

    pub async fn lookup_gateway_ip_from_nym_api(&self, gateway_identity: &str) -> Result<IpAddr> {
        debug!("Fetching gateway ip from nym-api...");
        let mut ips = self
            .api_client
            .get_all_described_nodes()
            .await
            .map_err(|e| Error::NymApi {
                source: Box::new(e),
            })?
            .iter()
            .find_map(|node| {
                if node
                    .description
                    .host_information
                    .keys
                    .ed25519
                    .to_base58_string()
                    == gateway_identity
                {
                    Some(node.description.host_information.ip_address.clone())
                } else {
                    None
                }
            })
            .ok_or(Error::RequestedGatewayIdNotFound(
                gateway_identity.to_string(),
            ))?;

        if ips.is_empty() {
            // nym-api should forbid this from ever happening, but we don't want to accidentally panic
            // if this assumption fails
            warn!("somehow {gateway_identity} hasn't provided any ip addresses!");
            return Err(Error::RequestedGatewayIdNotFound(
                gateway_identity.to_string(),
            ));
        }

        debug!("found the following ips for {gateway_identity}: {ips:?}");
        if ips.len() == 1 {
            // SAFETY: the vector is not empty, so unwrap is fine
            #[allow(clippy::unwrap_used)]
            Ok(ips.pop().unwrap())
        } else {
            // chose a random one if there's more than one
            // SAFETY: the vector is not empty, so unwrap is fine
            let mut rng = thread_rng();
            let ip = ips.choose(&mut rng).ok_or_else(|| Error::NoIpsAvailable)?;
            Ok(*ip)
        }
    }

    pub async fn lookup_all_gateways_from_nym_api(&self) -> Result<GatewayList> {
        let skimmed_gateways = self.lookup_skimmed_gateways().await?;
        let key_rotation_id = skimmed_gateways.metadata.rotation_id;

        let mut gateways = self
            .lookup_described_nodes()
            .await?
            .into_iter()
            .filter(|node| node.description.declared_role.entry)
            .filter_map(|gw| {
                Gateway::try_from_node_description(gw, key_rotation_id)
                    .inspect_err(|err| error!("Failed to parse gateway: {err}"))
                    .ok()
            })
            .collect::<Vec<_>>();
        append_performance(&mut gateways, skimmed_gateways.nodes);
        filter_on_mixnet_min_performance(&mut gateways, &self.min_gateway_performance);
        update_thresholds(&mut gateways, self.mix_score_thresholds);
        Ok(GatewayList::new(None, gateways))
    }

    pub async fn lookup_all_nymnodes(&self) -> Result<NymNodeList> {
        let skimmed_nodes = self.lookup_skimmed_nodes().await?;
        let key_rotation_id = skimmed_nodes.metadata.rotation_id;

        let mut nodes = self
            .lookup_described_nodes()
            .await?
            .into_iter()
            .filter_map(|gw| {
                Gateway::try_from_node_description(gw, key_rotation_id)
                    .inspect_err(|err| error!("Failed to parse node: {err}"))
                    .ok()
            })
            .collect::<Vec<_>>();
        append_performance(&mut nodes, skimmed_nodes.nodes);
        filter_on_mixnet_min_performance(&mut nodes, &self.min_gateway_performance);
        update_thresholds(&mut nodes, self.mix_score_thresholds);
        Ok(GatewayList::new(None, nodes))
    }

    pub async fn lookup_gateways_from_nym_api(&self, gw_type: GatewayType) -> Result<GatewayList> {
        match gw_type {
            GatewayType::MixnetEntry => self.lookup_entry_gateways_from_nym_api().await,
            GatewayType::MixnetExit => self.lookup_exit_gateways_from_nym_api().await,
            GatewayType::Wg => self.lookup_vpn_gateways_from_nym_api().await,
        }
    }

    async fn lookup_entry_gateways_from_nym_api(&self) -> Result<GatewayList> {
        self.lookup_all_gateways_from_nym_api().await
    }

    async fn lookup_exit_gateways_from_nym_api(&self) -> Result<GatewayList> {
        self.lookup_all_gateways_from_nym_api()
            .await
            .map(GatewayList::into_exit_gateways)
    }

    async fn lookup_vpn_gateways_from_nym_api(&self) -> Result<GatewayList> {
        self.lookup_all_gateways_from_nym_api()
            .await
            .map(GatewayList::into_vpn_gateways)
    }

    pub async fn lookup_gateway_ip(&self, gateway_identity: &str) -> Result<IpAddr> {
        self.lookup_gateway_ip_from_nym_api(gateway_identity).await
    }

    pub async fn lookup_all_gateways(&self) -> Result<GatewayList> {
        self.lookup_all_gateways_from_nym_api().await
    }

    pub async fn lookup_gateways(&self, gw_type: GatewayType) -> Result<GatewayList> {
        self.lookup_gateways_from_nym_api(gw_type).await
    }
}

fn append_performance(
    gateways: &mut [Gateway],
    basic_gw: Vec<nym_validator_client::nym_nodes::SkimmedNode>,
) {
    debug!("Appending mixnet_performance to gateways");
    for gateway in gateways.iter_mut() {
        if let Some(basic_gw) = basic_gw
            .iter()
            .find(|bgw| bgw.ed25519_identity_pubkey == gateway.identity())
        {
            gateway.mixnet_performance = Some(basic_gw.performance);
        } else {
            tracing::warn!(
                "Failed to append mixnet_performance, node {} not found among the skimmed nodes",
                gateway.identity()
            );
        }
    }
}

fn filter_on_mixnet_min_performance(
    gateways: &mut Vec<Gateway>,
    min_gateway_performance: &Option<GatewayMinPerformance>,
) {
    if let Some(min_performance) = min_gateway_performance
        && let Some(mixnet_min_performance) = min_performance.mixnet_min_performance
    {
        tracing::debug!(
            "Filtering gateways based on mixnet_min_performance: {:?}",
            min_performance
        );
        gateways.retain(|gateway| {
            gateway.mixnet_performance.unwrap_or_default() >= mixnet_min_performance
        });
    }
}

fn update_thresholds(gateways: &mut [Gateway], mix_score_thresholds: Option<ScoreThresholds>) {
    for gateway in gateways.iter_mut() {
        gateway.update_to_new_thresholds(mix_score_thresholds);
    }
}
