// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_explorer_api_requests::Location;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

pub(crate) type LocationCache<T> = HashMap<T, LocationCacheItem>;

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

impl LocationCacheItem {
    pub(crate) fn new_from_location(location: Option<Location>) -> Self {
        LocationCacheItem {
            location,
            valid_until: SystemTime::now() + Duration::from_secs(60 * 60 * 24), // valid for 1 day
        }
    }
}
