use std::{collections::HashMap, fmt};

use log::{debug, error, info};
use nym_explorer_api_requests::PrettyDetailedMixNodeBond;
use nym_topology::{
    mix::Layer,
    nym_topology_from_detailed,
    provider_trait::{async_trait, TopologyProvider},
    NymTopology,
};
use nym_validator_client::client::{MixId, MixNodeDetails};
use url::Url;

const MIN_NODES_PER_LAYER: usize = 2;
const EXPLORER_API_MIXNODES_URL: &str = "https://explorer.nymtech.net/api/v1/mix-nodes";

async fn fetch_mixnodes_from_explorer_api() -> Option<Vec<PrettyDetailedMixNodeBond>> {
    reqwest::get(EXPLORER_API_MIXNODES_URL)
        .await
        .ok()?
        .json::<Vec<PrettyDetailedMixNodeBond>>()
        .await
        .ok()
}

#[derive(Hash, PartialEq, Eq)]
pub enum CountryGroup {
    Europe,
    NorthAmerica,
    SouthAmerica,
    Oceania,
    Asia,
    Africa,
    Unknown,
}

impl fmt::Display for CountryGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use CountryGroup::*;
        match self {
            Europe => write!(f, "EU"),
            NorthAmerica => write!(f, "NA"),
            SouthAmerica => write!(f, "SA"),
            Oceania => write!(f, "OC"),
            Asia => write!(f, "AS"),
            Africa => write!(f, "AF"),
            Unknown => write!(f, "Unknown"),
        }
    }
}

// We map contry codes into group, which initially are continent codes to a first approximation,
// but we do it manually to reserve the right to tweak this distribution for our purposes.
impl From<&str> for CountryGroup {
    fn from(country_code: &str) -> Self {
        use CountryGroup::*;
        match country_code {
            // Europe
            "AT" => Europe,
            "BG" => Europe,
            "CH" => Europe,
            "CY" => Europe,
            "CZ" => Europe,
            "DE" => Europe,
            "DK" => Europe,
            "ES" => Europe,
            "FI" => Europe,
            "FR" => Europe,
            "GB" => Europe,
            "GR" => Europe,
            "IE" => Europe,
            "IT" => Europe,
            "LT" => Europe,
            "LU" => Europe,
            "LV" => Europe,
            "MD" => Europe,
            "MT" => Europe,
            "NL" => Europe,
            "NO" => Europe,
            "PL" => Europe,
            "RO" => Europe,
            "SE" => Europe,
            "SK" => Europe,
            "TR" => Europe,
            "UA" => Europe,

            // North America
            "CA" => NorthAmerica,
            "MX" => NorthAmerica,
            "US" => NorthAmerica,

            // South America
            "AR" => SouthAmerica,
            "BR" => SouthAmerica,
            "CL" => SouthAmerica,
            "CO" => SouthAmerica,
            "CR" => SouthAmerica,
            "GT" => SouthAmerica,

            // Oceania
            "AU" => Oceania,

            // Asia
            "AM" => Asia,
            "BH" => Asia,
            "CN" => Asia,
            "GE" => Asia,
            "HK" => Asia,
            "ID" => Asia,
            "IL" => Asia,
            "IN" => Asia,
            "JP" => Asia,
            "KH" => Asia,
            "KR" => Asia,
            "KZ" => Asia,
            "MY" => Asia,
            "RU" => Asia,
            "SG" => Asia,
            "TH" => Asia,
            "VN" => Asia,

            // Africa
            "SC" => Africa,
            "UG" => Africa,
            "ZA" => Africa,

            _ => {
                info!("Unknown country code: {}", country_code);
                Unknown
            }
        }
    }
}

impl CountryGroup {
    #[allow(unused)]
    fn known(self) -> Option<CountryGroup> {
        use CountryGroup::*;
        match self {
            Europe | NorthAmerica | SouthAmerica | Oceania | Asia | Africa => Some(self),
            Unknown => None,
        }
    }
}

fn group_mixnodes_by_country_code(
    mixnodes: Vec<PrettyDetailedMixNodeBond>,
) -> HashMap<CountryGroup, Vec<MixId>> {
    mixnodes
        .into_iter()
        .fold(HashMap::<CountryGroup, Vec<MixId>>::new(), |mut acc, m| {
            if let Some(ref location) = m.location {
                let country_code = location.two_letter_iso_country_code.clone();
                let group_code = CountryGroup::from(country_code.as_str());
                let mixnodes = acc.entry(group_code).or_insert_with(Vec::new);
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
        if count < MIN_NODES_PER_LAYER {
            error!("There are only {} mixnodes in layer {:?}", count, layer);
            return Err(());
        }
    }
    Ok(())
}

pub struct GeoAwareTopologyProvider {
    validator_client: nym_validator_client::client::NymApiClient,
    filter_on: CountryGroup,
}

impl GeoAwareTopologyProvider {
    pub fn new(nym_api_url: Url, filter_on: CountryGroup) -> GeoAwareTopologyProvider {
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
