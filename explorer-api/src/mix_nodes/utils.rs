// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::location::GeoLocation;
use isocountry::CountryCode;

#[allow(dead_code)]
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
