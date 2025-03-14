use crate::config::GroupBy;
use log::{debug, error};
use nym_explorer_client::{ExplorerClient, PrettyDetailedMixNodeBond};
use nym_network_defaults::var_names::EXPLORER_API;
use nym_topology::{
    provider_trait::{async_trait, TopologyProvider},
    NymTopology,
};
use nym_validator_client::client::NodeId;
use rand::{prelude::SliceRandom, thread_rng};
use std::collections::HashMap;
use tap::TapOptional;
use url::Url;

pub use nym_country_group::CountryGroup;

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
) -> HashMap<CountryGroup, Vec<NodeId>> {
    mixnodes
        .into_iter()
        .fold(HashMap::<CountryGroup, Vec<NodeId>>::new(), |mut acc, m| {
            if let Some(ref location) = m.location {
                let country_code = location.two_letter_iso_country_code.clone();
                let group_code = CountryGroup::new(country_code.as_str());
                let mixnodes = acc.entry(group_code).or_default();
                mixnodes.push(m.mix_id);
            }
            acc
        })
}

fn log_mixnode_distribution(mixnodes: &HashMap<CountryGroup, Vec<NodeId>>) {
    let mixnode_distribution = mixnodes
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v.len()))
        .collect::<Vec<_>>()
        .join(", ");
    debug!("Mixnode distribution - {}", mixnode_distribution);
}

fn check_layer_integrity(topology: NymTopology) -> Result<(), ()> {
    if topology.ensure_minimally_routable().is_err() {
        error!("Layer is missing in topology!");
        return Err(());
    }
    Ok(())
}

#[deprecated(note = "use NymApiTopologyProvider instead as explorer API will soon be removed")]
pub struct GeoAwareTopologyProvider {
    validator_client: nym_validator_client::client::NymApiClient,
    filter_on: GroupBy,
}

#[allow(deprecated)]
impl GeoAwareTopologyProvider {
    pub fn new(mut nym_api_urls: Vec<Url>, filter_on: GroupBy) -> GeoAwareTopologyProvider {
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
        }
    }

    async fn get_topology(&self) -> Option<NymTopology> {
        let rewarded_set = self
            .validator_client
            .get_current_rewarded_set()
            .await
            .inspect_err(|err| error!("failed to get current rewarded set: {err}"))
            .ok()?;

        let mut topology = NymTopology::new_empty(rewarded_set);

        let mixnodes = match self
            .validator_client
            .get_all_basic_active_mixing_assigned_nodes()
            .await
        {
            Err(err) => {
                error!("failed to get network mixnodes - {err}");
                return None;
            }
            Ok(mixes) => mixes,
        };

        let gateways = match self
            .validator_client
            .get_all_basic_entry_assigned_nodes()
            .await
        {
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
            .filter(|m| filtered_mixnode_ids.contains(&m.node_id))
            .collect::<Vec<_>>();

        topology.add_skimmed_nodes(&mixnodes);
        topology.add_skimmed_nodes(&gateways);

        // TODO: return real error type
        check_layer_integrity(topology.clone()).ok()?;

        Some(topology)
    }
}

#[allow(deprecated)]
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl TopologyProvider for GeoAwareTopologyProvider {
    // this will be manually refreshed on a timer specified inside mixnet client config
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_topology().await
    }
}

#[allow(deprecated)]
#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl TopologyProvider for GeoAwareTopologyProvider {
    // this will be manually refreshed on a timer specified inside mixnet client config
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_topology().await
    }
}
