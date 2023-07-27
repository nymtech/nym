use std::collections::HashMap;

use log::{debug, error};
use nym_explorer_api_requests::PrettyDetailedMixNodeBond;
use nym_topology::{
    mix::Layer,
    nym_topology_from_detailed,
    provider_trait::{async_trait, TopologyProvider},
    NymTopology,
};
use nym_validator_client::client::MixId;
use url::Url;

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
        let mixnodes_from_explorer_api =
            reqwest::get("https://explorer.nymtech.net/api/v1/mix-nodes")
                .await
                .unwrap()
                .json::<Vec<PrettyDetailedMixNodeBond>>()
                .await
                .unwrap();

        // Partition mixnodes_from_explorer_api according to the value of two_letter_iso_country_code
        let mixnodes_from_explorer_api_by_continent = mixnodes_from_explorer_api.into_iter().fold(
            HashMap::<String, Vec<MixId>>::new(),
            |mut acc, m| {
                if let Some(ref location) = m.location {
                    let country_code = location.two_letter_iso_country_code.clone();
                    if let Some(continent_code) = country_code_to_continent_code(&country_code) {
                        let mixnodes = acc.entry(continent_code).or_insert_with(Vec::new);
                        mixnodes.push(m.mix_id);
                    }
                }
                acc
            },
        );

        // Create a string with the number of mixnodes per continent.
        let mixnodes_from_explorer_api_by_continent_string =
            mixnodes_from_explorer_api_by_continent
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v.len()))
                .collect::<Vec<_>>()
                .join(", ");
        debug!(
            "Mixnode distribution - {}",
            mixnodes_from_explorer_api_by_continent_string
        );

        // Filter mixnodes so that only the items that also exist in the mixnodes_from_explorer_api_by_continent for the key given by filter_on.
        let mixnodes = mixnodes
            .into_iter()
            .filter(|m| {
                if let Some(ids) = mixnodes_from_explorer_api_by_continent.get(&self.filter_on) {
                    ids.contains(&m.mix_id())
                } else {
                    // If the key is not setup, or no mixnodes exist for the key, then return false.
                    false
                }
            })
            .collect::<Vec<_>>();

        // Check layer distribution
        let mut layer_counts = mixnodes.iter().map(|m| m.layer()).fold(
            HashMap::<Layer, usize>::new(),
            |mut acc, layer| {
                let count = acc.entry(layer).or_insert(0);
                *count += 1;
                acc
            },
        );

        // and check the integrity
        for layer in &[Layer::One, Layer::Two, Layer::Three] {
            layer_counts.entry(*layer).or_insert(0);
            let count = layer_counts[layer];
            if count < 2 {
                error!("There are only {} mixnodes in layer {:?}", count, layer);
                return None;
            }
        }

        Some(nym_topology_from_detailed(mixnodes, gateways))
    }
}

// We map contry codes to continent codes, but we do it manually to reserve the right to tweak this
// distribution for our purposes.
// Also, at the time of writing I didn't find a simple crate that did this mapping...
fn country_code_to_continent_code(country_code: &str) -> Option<String> {
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

#[async_trait]
impl TopologyProvider for GeoAwareTopologyProvider {
    // this will be manually refreshed on a timer specified inside mixnet client config
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_topology().await
    }
}
