// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use isocountry::CountryCode;
use nym_mixnet_contract_common::families::FamilyHead;
use rand::Rng;
use rand_pcg::Pcg64;
use rand_seeder::Seeder;

use crate::location::GeoLocation;

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

// We don't need numerical IDs anywhere, so to avoid modifying the contract storage again and
//  since this is for explorer ergonomics, it will generate a deterministic random u16 based on the family Identity.
pub(crate) fn family_numerical_id(fh: &FamilyHead) -> u16 {
    let mut rng: Pcg64 = Seeder::from(fh.identity()).make_rng();
    rng.gen()
}
