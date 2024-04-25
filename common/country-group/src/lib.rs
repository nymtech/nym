// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::info;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum CountryGroup {
    Europe,
    NorthAmerica,
    SouthAmerica,
    Oceania,
    Asia,
    Africa,
    Unknown,
}

impl CountryGroup {
    // We map country codes into group, which initially are continent codes to a first approximation,
    // but we do it manually to reserve the right to tweak this distribution for our purposes.
    // NOTE: I did this quickly, and it's not a complete list of all countries, but only those that
    // were present in the network at the time. Please add more as needed.
    pub fn new(country_code: &str) -> Self {
        let country_code = country_code.to_uppercase();
        use CountryGroup::*;
        match country_code.as_ref() {
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

            // And group level codes work too
            "EU" => Europe,
            "NA" => NorthAmerica,
            "SA" => SouthAmerica,
            "OC" => Oceania,
            "AS" => Asia,
            "AF" => Africa,

            // And some aliases
            "EUROPE" => Europe,
            "NORTHAMERICA" => NorthAmerica,
            "SOUTHAMERICA" => SouthAmerica,
            "OCEANIA" => Oceania,
            "ASIA" => Asia,
            "AFRICA" => Africa,

            _ => {
                info!("Unknown country code: {country_code}");
                Unknown
            }
        }
    }
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

impl std::str::FromStr for CountryGroup {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let group = CountryGroup::new(s);
        if group == CountryGroup::Unknown {
            Err(())
        } else {
            Ok(group)
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
