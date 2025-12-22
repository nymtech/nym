//! Topology fetching from nym-api
//!
//! Queries nym-api for active mix nodes and gateways,
//! builds routes for Sphinx packet construction.

use anyhow::{Context, Result, anyhow, bail};
use nym_api_requests::nym_nodes::SkimmedNode;
use nym_crypto::asymmetric::ed25519;
use nym_http_api_client::UserAgent;
use nym_sphinx_types::Node as SphinxNode;
use nym_topology::{NymTopology, NymTopologyMetadata};
use nym_validator_client::nym_api::NymApiClientExt;
use rand::prelude::IteratorRandom;
use rand::{CryptoRng, Rng};
use std::net::SocketAddr;
use tracing::{debug, info};
use url::Url;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Gateway information for LP connection
#[derive(Debug, Clone)]
pub struct GatewayInfo {
    pub identity: ed25519::PublicKey,
    pub sphinx_key: nym_crypto::asymmetric::x25519::PublicKey,
    /// Mix host (IP:port for Sphinx mixing)
    pub mix_host: SocketAddr,
    /// LP control address (IP:41264)
    pub lp_address: SocketAddr,
}

/// Topology for routing Sphinx packets
pub struct SpeedtestTopology {
    topology: NymTopology,
    /// Entry gateways available for LP connection
    gateways: Vec<GatewayInfo>,
}

impl SpeedtestTopology {
    /// Fetch network topology from nym-api
    pub async fn fetch(nym_api: &Url) -> Result<Self> {
        info!("Fetching topology from {}", nym_api);

        let user_agent = UserAgent {
            application: "nym-lp-speedtest".to_string(),
            version: VERSION.to_string(),
            platform: std::env::consts::OS.to_string(),
            git_commit: "unknown".to_string(),
        };
        let api_client = nym_http_api_client::Client::builder(nym_api.clone())
            .context("malformed nym api url")?
            .with_user_agent(user_agent)
            .build()
            .context("failed to build nym api client")?;

        // Fetch mixing nodes in active set
        debug!("Fetching active mixing nodes...");
        let mixing_nodes = api_client
            .get_all_basic_active_mixing_assigned_nodes_with_metadata()
            .await
            .context("failed to fetch mixing nodes")?;

        info!(
            "Fetched {} mixing nodes",
            mixing_nodes.nodes.len()
        );

        // Fetch entry gateways
        debug!("Fetching entry gateways...");
        let entry_gateways = api_client
            .get_all_basic_entry_assigned_nodes_with_metadata()
            .await
            .context("failed to fetch entry gateways")?;

        info!(
            "Fetched {} entry gateways",
            entry_gateways.nodes.len()
        );

        // Get rewarded set info
        debug!("Fetching rewarded set...");
        let rewarded_set = api_client
            .get_rewarded_set()
            .await
            .context("failed to fetch rewarded set")?;

        // Build NymTopology
        let metadata = NymTopologyMetadata::new(
            mixing_nodes.metadata.rotation_id,
            rewarded_set.epoch_id,
            time::OffsetDateTime::now_utc(),
        );

        // Convert RewardedSetResponse -> EpochRewardedSet (impl Into<CachedEpochRewardedSet>)
        let epoch_rewarded_set: nym_topology::EpochRewardedSet = rewarded_set.into();

        let mut topology = NymTopology::new(metadata, epoch_rewarded_set, vec![]);

        // Add mixing nodes
        topology.add_skimmed_nodes(&mixing_nodes.nodes);

        // Add entry gateways
        topology.add_skimmed_nodes(&entry_gateways.nodes);

        // Extract gateway info for LP connections
        let gateways = entry_gateways
            .nodes
            .iter()
            .filter_map(|node| gateway_info_from_skimmed(node).ok())
            .collect::<Vec<_>>();

        if gateways.is_empty() {
            bail!("No entry gateways available for LP connection");
        }

        info!("Built topology with {} usable gateways", gateways.len());

        Ok(SpeedtestTopology { topology, gateways })
    }

    /// Get a specific gateway by identity string
    pub fn gateway_by_identity(&self, identity: &str) -> Result<&GatewayInfo> {
        let identity_key: ed25519::PublicKey = identity
            .parse()
            .context("invalid gateway identity")?;

        self.gateways
            .iter()
            .find(|g| g.identity == identity_key)
            .ok_or_else(|| anyhow!("gateway {} not found in topology", identity))
    }

    /// Select a random entry gateway
    pub fn random_gateway<R: Rng + ?Sized>(&self, rng: &mut R) -> Result<&GatewayInfo> {
        self.gateways
            .iter()
            .choose(rng)
            .ok_or_else(|| anyhow!("no gateways available"))
    }

    /// Build a random 3-hop route through the mixnet to the given destination gateway.
    /// Returns (route, destination_sphinx_node) where route has 3 mix nodes.
    pub fn random_route_to_gateway<R: Rng + CryptoRng + ?Sized>(
        &self,
        rng: &mut R,
        gateway: &GatewayInfo,
    ) -> Result<Vec<SphinxNode>> {
        // Build route to the gateway's identity
        let route = self
            .topology
            .random_route_to_egress(rng, gateway.identity.into(), true)
            .context("failed to build route to gateway")?;

        if route.is_empty() {
            bail!("empty route returned from topology");
        }

        Ok(route)
    }

    /// Get number of available gateways
    pub fn gateway_count(&self) -> usize {
        self.gateways.len()
    }

    /// Get all gateways
    pub fn gateways(&self) -> &[GatewayInfo] {
        &self.gateways
    }
}

/// Extract gateway info for LP connections from a SkimmedNode
fn gateway_info_from_skimmed(node: &SkimmedNode) -> Result<GatewayInfo> {
    let first_ip = node
        .ip_addresses
        .first()
        .ok_or_else(|| anyhow!("node has no IP addresses"))?;

    // LP default control port
    const LP_CONTROL_PORT: u16 = 41264;

    Ok(GatewayInfo {
        identity: node.ed25519_identity_pubkey,
        sphinx_key: node.x25519_sphinx_pubkey,
        mix_host: SocketAddr::new(*first_ip, node.mix_port),
        lp_address: SocketAddr::new(*first_ip, LP_CONTROL_PORT),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn test_fetch_topology() {
        let nym_api = Url::parse("https://validator.nymtech.net/api").unwrap();
        let topology = SpeedtestTopology::fetch(&nym_api).await.unwrap();

        assert!(topology.gateway_count() > 0);
        println!("Found {} gateways", topology.gateway_count());

        let mut rng = rand::rng();
        let gateway = topology.random_gateway(&mut rng).unwrap();
        println!("Selected gateway: {:?}", gateway.identity);

        let route = topology.random_route_to_gateway(&mut rng, gateway).unwrap();
        println!("Route has {} hops", route.len());
    }
}
