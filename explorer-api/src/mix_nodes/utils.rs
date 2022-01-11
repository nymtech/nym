// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_nodes::location::GeoLocation;
use isocountry::CountryCode;
use network_defaults::{
    default_api_endpoints, default_nymd_endpoints, DEFAULT_MIXNET_CONTRACT_ADDRESS,
};
use validator_client::nymd::QueryNymdClient;

pub(crate) fn map_2_letter_to_3_letter_country_code(geo: &GeoLocation) -> String {
    match CountryCode::for_alpha2(&geo.country_code) {
        Ok(three_letter_country_code) => three_letter_country_code.alpha3().to_string(),
        Err(_e) => {
            warn!(
                "❌ Oh no! map_2_letter_to_3_letter_country_code failed for '{:#?}'",
                geo
            );
            "???".to_string()
        }
    }
}

pub(crate) fn new_nymd_client() -> validator_client::Client<QueryNymdClient> {
    let mixnet_contract = DEFAULT_MIXNET_CONTRACT_ADDRESS.to_string();
    let nymd_url = default_nymd_endpoints()[0].clone();
    let api_url = default_api_endpoints()[0].clone();

    let client_config = validator_client::Config::new(
        nymd_url,
        api_url,
        Some(mixnet_contract.parse().unwrap()),
        None,
    );

    validator_client::Client::new_query(client_config).expect("Failed to connect to nymd!")
}
