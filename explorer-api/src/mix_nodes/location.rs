// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_nodes::utils::map_2_letter_to_3_letter_country_code;
use mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

pub(crate) type LocationCache = HashMap<NodeId, LocationCacheItem>;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct GeoLocation {
    pub(crate) ip: String,
    pub(crate) country_code: String,
    pub(crate) country_name: String,
    pub(crate) region_code: String,
    pub(crate) region_name: String,
    pub(crate) city: String,
    pub(crate) zip_code: String,
    pub(crate) time_zone: String,
    pub(crate) latitude: f32,
    pub(crate) longitude: f32,
    pub(crate) metro_code: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LocationCacheItem {
    pub(crate) location: Option<Location>,
    pub(crate) valid_until: SystemTime,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize)]
pub(crate) struct Location {
    pub(crate) two_letter_iso_country_code: String,
    pub(crate) three_letter_iso_country_code: String,
    pub(crate) country_name: String,
    pub(crate) lat: f32,
    pub(crate) lng: f32,
}

impl Location {
    pub(crate) fn new(geo_location: GeoLocation) -> Self {
        let three_letter_iso_country_code = map_2_letter_to_3_letter_country_code(&geo_location);
        Location {
            country_name: geo_location.country_name,
            two_letter_iso_country_code: geo_location.country_code,
            three_letter_iso_country_code,
            lat: geo_location.latitude,
            lng: geo_location.longitude,
        }
    }
}

impl LocationCacheItem {
    pub(crate) fn new_from_location(location: Option<Location>) -> Self {
        LocationCacheItem {
            location,
            valid_until: SystemTime::now() + Duration::from_secs(60 * 60 * 24), // valid for 1 day
        }
    }
}
