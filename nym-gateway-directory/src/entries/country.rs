// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::Location;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Country {
    iso_code: String,
}

impl Country {
    pub fn iso_code(&self) -> &str {
        &self.iso_code
    }
}

impl From<nym_vpn_api_client::response::NymDirectoryCountry> for Country {
    fn from(country: nym_vpn_api_client::response::NymDirectoryCountry) -> Self {
        Self {
            iso_code: country.iso_code().to_string(),
        }
    }
}

impl From<Location> for Country {
    fn from(location: Location) -> Self {
        Self {
            iso_code: location.two_letter_iso_country_code,
        }
    }
}
