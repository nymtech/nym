// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::bail;
use ipinfo::IpDetails;
use nym_geolocation_contract_common::ContractConfig;
use nym_geolocation_contract_common::payload::{Asn, Coordinates, Location};
use nym_validator_client::nyxd::contract_traits::GeolocationQueryClient;

/// Read the contract's tunables once, at startup.
///
/// Not re-read per request: changing them takes an admin transaction, which is a multisig, and
/// restarting the binary alongside one is no burden.
pub(super) async fn retrieve_contract_config<C>(client: &C) -> anyhow::Result<ContractConfig>
where
    C: GeolocationQueryClient + Send + Sync,
{
    Ok(client.get_geolocation_config().await?.config)
}

fn parse_coordinates(raw: String) -> Option<Coordinates> {
    if raw.is_empty() {
        return None;
    }
    raw.split_once(',')
        .and_then(|(lat, lon)| lat.parse::<f64>().ok().zip(lon.parse::<f64>().ok()))
        .map(|(latitude, longitude)| Coordinates {
            latitude,
            longitude,
        })
}

// can't use From trait as both of these are from external crates
pub(crate) fn ip_info_to_location(response: IpDetails) -> anyhow::Result<Location> {
    // the provider answers with empty fields rather than an error for an address it cannot
    // place - a bogon or a reserved range being the usual cause. That is an absent location,
    // not a location: an entry asserting country "" is a committed, provable claim that says
    // nothing, and nothing reading the contract can tell it from a real answer
    if response.country.is_empty() {
        bail!("no country was determined for the address")
    }

    Ok(Location {
        two_letter_iso_country_code: response.country,
        coordinates: parse_coordinates(response.loc),
        city: response.city,
        region: response.region,
        org: response.org.unwrap_or_default(),
        postal: response.postal.unwrap_or_default(),
        timezone: response.timezone.unwrap_or_default(),
        asn: response.asn.map(|asn_details| Asn {
            asn: asn_details.asn,
            name: asn_details.name,
            domain: asn_details.domain,
            route: asn_details.route,
            kind: asn_details.asn_type,
        }),
    })
}
