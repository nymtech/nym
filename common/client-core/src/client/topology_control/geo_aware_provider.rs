use std::collections::HashMap;

use log::{debug, error};
use nym_explorer_api_requests::PrettyDetailedMixNodeBond;
use nym_topology::{
    mix::Layer,
    nym_topology_from_detailed,
    provider_trait::{async_trait, TopologyProvider},
    NymTopology,
};
use nym_validator_client::client::{MixId, MixNodeDetails};
use url::Url;

const EXPLORER_API_MIXNODES_URL: &str = "https://explorer.nymtech.net/api/v1/mix-nodes";

async fn fetch_mixnodes_from_explorer_api() -> Option<Vec<PrettyDetailedMixNodeBond>> {
    reqwest::get(EXPLORER_API_MIXNODES_URL)
        .await
        .ok()?
        .json::<Vec<PrettyDetailedMixNodeBond>>()
        .await
        .ok()
}

// We map contry codes into group, which initially are continent codes to a first approximation,
// but we do it manually to reserve the right to tweak this distribution for our purposes.
// TODO: replace String with enum
fn country_code_to_group_code(country_code: &str) -> Option<String> {
    match country_code {
        // Europe
        "AT" => Some("EU".to_string()),
        "BG" => Some("EU".to_string()),
        "CH" => Some("EU".to_string()),
        "CY" => Some("EU".to_string()),
        "CZ" => Some("EU".to_string()),
        "DE" => Some("EU".to_string()),
        "DK" => Some("EU".to_string()),
        "ES" => Some("EU".to_string()),
        "FI" => Some("EU".to_string()),
        "FR" => Some("EU".to_string()),
        "GB" => Some("EU".to_string()),
        "GR" => Some("EU".to_string()),
        "IE" => Some("EU".to_string()),
        "IT" => Some("EU".to_string()),
        "LT" => Some("EU".to_string()),
        "LU" => Some("EU".to_string()),
        "LV" => Some("EU".to_string()),
        "MD" => Some("EU".to_string()),
        "MT" => Some("EU".to_string()),
        "NL" => Some("EU".to_string()),
        "NO" => Some("EU".to_string()),
        "PL" => Some("EU".to_string()),
        "RO" => Some("EU".to_string()),
        "SE" => Some("EU".to_string()),
        "SK" => Some("EU".to_string()),
        "TR" => Some("EU".to_string()),
        "UA" => Some("EU".to_string()),

        // North America
        "CA" => Some("NA".to_string()),
        "MX" => Some("NA".to_string()),
        "US" => Some("NA".to_string()),

        // South America
        "AR" => Some("SA".to_string()),
        "BR" => Some("SA".to_string()),
        "CL" => Some("SA".to_string()),
        "CO" => Some("SA".to_string()),
        "CR" => Some("SA".to_string()),
        "GT" => Some("SA".to_string()),

        // Oceania
        "AU" => Some("OC".to_string()),

        // Asia
        "AM" => Some("AS".to_string()),
        "BH" => Some("AS".to_string()),
        "CN" => Some("AS".to_string()),
        "GE" => Some("AS".to_string()),
        "HK" => Some("AS".to_string()),
        "ID" => Some("AS".to_string()),
        "IL" => Some("AS".to_string()),
        "IN" => Some("AS".to_string()),
        "JP" => Some("AS".to_string()),
        "KH" => Some("AS".to_string()),
        "KR" => Some("AS".to_string()),
        "KZ" => Some("AS".to_string()),
        "MY" => Some("AS".to_string()),
        "RU" => Some("AS".to_string()),
        "SG" => Some("AS".to_string()),
        "TH" => Some("AS".to_string()),
        "VN" => Some("AS".to_string()),

        // Africa
        "SC" => Some("AF".to_string()),
        "UG" => Some("AF".to_string()),
        "ZA" => Some("AF".to_string()),

        _ => {
            println!("Unknown country code: {}", country_code);
            None
        }
    }
}

fn group_mixnodes_by_country_code(
    mixnodes: Vec<PrettyDetailedMixNodeBond>,
) -> HashMap<String, Vec<MixId>> {
    mixnodes
        .into_iter()
        .fold(HashMap::<String, Vec<MixId>>::new(), |mut acc, m| {
            if let Some(ref location) = m.location {
                let country_code = location.two_letter_iso_country_code.clone();
                if let Some(group_code) = country_code_to_group_code(&country_code) {
                    let mixnodes = acc.entry(group_code).or_insert_with(Vec::new);
                    mixnodes.push(m.mix_id);
                }
            }
            acc
        })
}

fn log_mixnode_distribution(mixnodes: &HashMap<String, Vec<MixId>>) {
    let mixnode_distribution = mixnodes
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v.len()))
        .collect::<Vec<_>>()
        .join(", ");
    debug!("Mixnode distribution - {}", mixnode_distribution);
}

fn count_mixnodes_per_layer(mixnodes: Vec<MixNodeDetails>) -> HashMap<Layer, usize> {
    mixnodes
        .iter()
        .map(|m| m.layer())
        .fold(HashMap::<Layer, usize>::new(), |mut acc, layer| {
            let count = acc.entry(layer).or_insert(0);
            *count += 1;
            acc
        })
}

fn check_layer_integrity(mixnodes: Vec<MixNodeDetails>) -> Result<(), ()> {
    let mut layer_counts = count_mixnodes_per_layer(mixnodes);
    for layer in &[Layer::One, Layer::Two, Layer::Three] {
        layer_counts.entry(*layer).or_insert(0);
        let count = layer_counts[layer];
        if count < 2 {
            error!("There are only {} mixnodes in layer {:?}", count, layer);
            return Err(());
        }
    }
    Ok(())
}

pub struct GeoAwareTopologyProvider {
    validator_client: nym_validator_client::client::NymApiClient,
    filter_on: String,
}

impl GeoAwareTopologyProvider {
    pub fn new(nym_api_url: Url, filter_on: String) -> GeoAwareTopologyProvider {
        GeoAwareTopologyProvider {
            validator_client: nym_validator_client::client::NymApiClient::new(nym_api_url),
            filter_on,
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
        let Some(mixnodes_from_explorer_api) = fetch_mixnodes_from_explorer_api().await else {
            error!("failed to get mixnodes from explorer-api");
            return None;
        };

        // Partition mixnodes_from_explorer_api according to the value of
        // two_letter_iso_country_code.
        // NOTE: we construct the full distribution here, but only use the one we're interested in.
        // The reason we this instead of a straight filter is that this opens up the possibility to
        // complement a small grouping with mixnodes from adjecent countries.
        let mixnode_distribution = group_mixnodes_by_country_code(mixnodes_from_explorer_api);
        log_mixnode_distribution(&mixnode_distribution);

        let Some(filtered_mixnode_ids) = mixnode_distribution.get(&self.filter_on) else {
            error!("no mixnodes found for: {}", self.filter_on);
            return None;
        };

        let mixnodes = mixnodes
            .into_iter()
            .filter(|m| filtered_mixnode_ids.contains(&m.mix_id()))
            .collect::<Vec<_>>();

        // TODO: return real error type
        check_layer_integrity(mixnodes.clone()).ok()?;

        Some(nym_topology_from_detailed(mixnodes, gateways))
    }
}

#[async_trait]
impl TopologyProvider for GeoAwareTopologyProvider {
    // this will be manually refreshed on a timer specified inside mixnet client config
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_topology().await
    }
}
