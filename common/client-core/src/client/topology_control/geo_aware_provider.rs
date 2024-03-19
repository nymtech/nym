use crate::config::GroupBy;
use log::{debug, error};
use nym_explorer_client::{ExplorerClient, PrettyDetailedMixNodeBond};
use nym_network_defaults::var_names::EXPLORER_API;
use nym_topology::{
    nym_topology_from_detailed,
    provider_trait::{async_trait, TopologyProvider},
    NymTopology,
};
use nym_validator_client::client::MixId;
use rand::{prelude::SliceRandom, thread_rng};
use std::collections::HashMap;
use tap::TapOptional;
use url::Url;

pub use nym_country_group::CountryGroup;

const MIN_NODES_PER_LAYER: usize = 1;

fn create_explorer_client() -> Option<ExplorerClient> {
    let Ok(explorer_api_url) = std::env::var(EXPLORER_API) else {
        error!("Missing EXPLORER_API");
        return None;
    };

    let Ok(explorer_api_url) = explorer_api_url.parse() else {
        error!("Failed to parse EXPLORER_API");
        return None;
    };

    log::debug!("Using explorer-api url: {}", explorer_api_url);
    let Ok(client) = nym_explorer_client::ExplorerClient::new(explorer_api_url) else {
        error!("Failed to create explorer-api client");
        return None;
    };

    Some(client)
}

fn group_mixnodes_by_country_code(
    mixnodes: Vec<PrettyDetailedMixNodeBond>,
) -> HashMap<CountryGroup, Vec<MixId>> {
    mixnodes
        .into_iter()
        .fold(HashMap::<CountryGroup, Vec<MixId>>::new(), |mut acc, m| {
            if let Some(ref location) = m.location {
                let country_code = location.two_letter_iso_country_code.clone();
                let group_code = CountryGroup::new(country_code.as_str());
                let mixnodes = acc.entry(group_code).or_default();
                mixnodes.push(m.mix_id);
            }
            acc
        })
}

fn log_mixnode_distribution(mixnodes: &HashMap<CountryGroup, Vec<MixId>>) {
    let mixnode_distribution = mixnodes
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v.len()))
        .collect::<Vec<_>>()
        .join(", ");
    debug!("Mixnode distribution - {}", mixnode_distribution);
}

fn check_layer_integrity(topology: NymTopology) -> Result<(), ()> {
    let mixes = topology.mixes();
    if mixes.keys().len() < 3 {
        error!("Layer is missing in topology!");
        return Err(());
    }
    for (layer, mixnodes) in mixes {
        debug!("Layer {:?} has {} mixnodes", layer, mixnodes.len());
        if mixnodes.len() < MIN_NODES_PER_LAYER {
            error!(
                "There are only {} mixnodes in layer {:?}",
                mixnodes.len(),
                layer
            );
            return Err(());
        }
    }
    Ok(())
}

pub struct GeoAwareTopologyProvider {
    validator_client: nym_validator_client::client::NymApiClient,
    filter_on: GroupBy,
    client_version: String,
}

impl GeoAwareTopologyProvider {
    pub fn new(
        mut nym_api_urls: Vec<Url>,
        client_version: String,
        filter_on: GroupBy,
    ) -> GeoAwareTopologyProvider {
        log::info!(
            "Creating geo-aware topology provider with filter on {}",
            filter_on
        );
        nym_api_urls.shuffle(&mut thread_rng());

        GeoAwareTopologyProvider {
            validator_client: nym_validator_client::client::NymApiClient::new(
                nym_api_urls[0].clone(),
            ),
            filter_on,
            client_version,
        }
    }

    async fn get_topology(&self) -> Option<NymTopology> {
        let mixnodes = match self.validator_client.get_cached_active_mixnodes().await {
            Err(err) => {
                error!("failed to get network mixnodes - {err}");
                return None;
            }
            Ok(mixes) => mixes,
        };

        let gateways = match self.validator_client.get_cached_gateways().await {
            Err(err) => {
                error!("failed to get network gateways - {err}");
                return None;
            }
            Ok(gateways) => gateways,
        };

        // Also fetch mixnodes cached by explorer-api, with the purpose of getting their
        // geolocation.
        debug!("Fetching mixnodes from explorer-api...");
        let explorer_client = create_explorer_client()?;
        let Ok(mixnodes_from_explorer_api) = explorer_client.get_mixnodes().await else {
            error!("failed to get mixnodes from explorer-api");
            return None;
        };

        debug!("Fetching gateways from explorer-api...");
        let Ok(gateways_from_explorer_api) = explorer_client.get_gateways().await else {
            error!("failed to get mixnodes from explorer-api");
            return None;
        };

        // Determine what we should filter around
        let filter_on = match self.filter_on {
            GroupBy::CountryGroup(group) => group,
            GroupBy::NymAddress(recipient) => {
                // Convert recipient into a country group by extracting out the gateway part and
                // using that as the country code.
                let gateway = recipient.gateway().to_base58_string();

                // Lookup the location of this gateway by using the location data from the
                // explorer-api
                let gateway_location = gateways_from_explorer_api
                    .iter()
                    .find(|g| g.gateway.identity_key == gateway)
                    .and_then(|g| g.location.clone())
                    .map(|location| location.two_letter_iso_country_code)
                    .tap_none(|| error!("No location found for the gateway: {}", gateway))?;
                debug!(
                    "Filtering on nym-address: {}, with location: {}",
                    recipient, gateway_location
                );

                CountryGroup::new(&gateway_location)
            }
        };
        debug!("Filter group: {}", filter_on);

        // Partition mixnodes_from_explorer_api according to the value of
        // two_letter_iso_country_code.
        // NOTE: we construct the full distribution here, but only use the one we're interested in.
        // The reason we this instead of a straight filter is that this opens up the possibility to
        // complement a small grouping with mixnodes from adjecent countries.
        let mixnode_distribution = group_mixnodes_by_country_code(mixnodes_from_explorer_api);
        log_mixnode_distribution(&mixnode_distribution);

        let Some(filtered_mixnode_ids) = mixnode_distribution.get(&filter_on) else {
            error!("no mixnodes found for: {}", filter_on);
            return None;
        };

        let mixnodes = mixnodes
            .into_iter()
            .filter(|m| filtered_mixnode_ids.contains(&m.mix_id()))
            .collect::<Vec<_>>();

        let topology = nym_topology_from_detailed(mixnodes, gateways)
            .filter_system_version(&self.client_version);

        // TODO: return real error type
        check_layer_integrity(topology.clone()).ok()?;

        Some(topology)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl TopologyProvider for GeoAwareTopologyProvider {
    // this will be manually refreshed on a timer specified inside mixnet client config
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_topology().await
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl TopologyProvider for GeoAwareTopologyProvider {
    // this will be manually refreshed on a timer specified inside mixnet client config
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_topology().await
    }
}
