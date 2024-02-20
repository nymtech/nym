// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::{append_ip_to_file, failed_ips_filepath};
use isocountry::CountryCode;
use log::warn;
use maxminddb::{geoip2::City, MaxMindDBError, Reader};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::Mutex;
use std::{
    net::{IpAddr, ToSocketAddrs},
    str::FromStr,
    sync::Arc,
};

const DEFAULT_DATABASE_PATH: &str = "./geo_ip/GeoLite2-City.mmdb";
const FAKE_PORT: u16 = 1234;

#[derive(Debug)]
pub enum GeoIpError {
    NoValidIP,
    InternalError,
}

// The current State implementation does not allow to fail on state
// creation, ie. returning Result<>. To avoid to use unwrap family,
// as a workaround, wrap the state inside an Option<>
// If Reader::open_readfile fails for some reason db will be set to None
// and an error will be logged.
pub(crate) struct GeoIp {
    pub(crate) db: Option<Reader<Vec<u8>>>,
    failed_addresses: FailedIpAddresses,
}

#[derive(Clone)]
pub(crate) struct ThreadsafeGeoIp(pub Arc<GeoIp>);

pub(crate) struct Location {
    /// two-letter country code (ISO 3166-1 alpha-2)
    pub(crate) iso_alpha2: String,
    /// three-letter country code (ISO 3166-1 alpha-3)
    pub(crate) iso_alpha3: String,
    /// English country short name (ISO 3166-1)
    pub(crate) name: String,
    pub(crate) latitude: Option<f64>,
    pub(crate) longitude: Option<f64>,
}

pub(crate) struct FailedIpAddresses {
    failed_ips: Mutex<HashSet<String>>,
}

impl FailedIpAddresses {
    pub fn new() -> Self {
        let mut failed_ips = HashSet::new();
        let file_path = failed_ips_filepath();

        if Path::new(&file_path).exists() {
            if let Ok(file) = File::open(&file_path) {
                let lines = io::BufReader::new(file).lines();
                for ip in lines.map_while(Result::ok) {
                    failed_ips.insert(ip);
                }
            }
        }

        FailedIpAddresses {
            failed_ips: Mutex::new(failed_ips),
        }
    }
}
impl From<Location> for nym_explorer_api_requests::Location {
    fn from(location: Location) -> Self {
        nym_explorer_api_requests::Location {
            country_name: location.name,
            two_letter_iso_country_code: location.iso_alpha2,
            three_letter_iso_country_code: location.iso_alpha3,
            latitude: location.latitude,
            longitude: location.longitude,
        }
    }
}

impl GeoIp {
    pub fn new() -> Self {
        let db_path = std::env::var("GEOIP_DB_PATH").unwrap_or_else(|e| {
            warn!(
                "Env variable GEOIP_DB_PATH is not set: {} - Fallback to {}",
                e, DEFAULT_DATABASE_PATH
            );
            DEFAULT_DATABASE_PATH.to_string()
        });
        let reader = Reader::open_readfile(&db_path)
            .map_err(|e| {
                error!("Fail to open GeoLite2 database file {}: {}", db_path, e);
            })
            .ok();

        let failed_addresses = FailedIpAddresses::new();

        GeoIp {
            db: reader,
            failed_addresses,
        }
    }

    fn handle_failed_ip(&self, address: &str) {
        if let Ok(mut failed_ips_guard) = self.failed_addresses.failed_ips.lock() {
            if failed_ips_guard.insert(address.to_string()) {
                append_ip_to_file(address);
            }
        } else {
            error!("Failed to acquire lock on failed_ips");
        }
    }

    pub fn query(&self, address: &str, port: Option<u16>) -> Result<Option<Location>, GeoIpError> {
        let p = port.unwrap_or(FAKE_PORT);
        let ip_result: Result<IpAddr, GeoIpError> = FromStr::from_str(address).or_else(|_| {
            debug!(
                "Fail to create IpAddr from {}. Trying using internal lookup...",
                &address
            );
            match (address, p).to_socket_addrs() {
                Ok(mut addrs) => {
                    if let Some(socket_addr) = addrs.next() {
                        let ip = socket_addr.ip();
                        debug!("Internal lookup succeeded, resolved ip: {}", ip);
                        Ok(ip)
                    } else {
                        debug!("Fail to resolve IP address from {}:{}", &address, p);
                        self.handle_failed_ip(address);
                        Err(GeoIpError::NoValidIP)
                    }
                }
                Err(_) => {
                    debug!("Fail to resolve IP address from {}:{}.", &address, p);
                    self.handle_failed_ip(address);
                    Err(GeoIpError::NoValidIP)
                }
            }
        });

        let ip = ip_result?;

        let result = self
            .db
            .as_ref()
            .ok_or_else(|| {
                error!("No registered GeoIP database");
                GeoIpError::InternalError
            })?
            .lookup::<City>(ip);
        match &result {
            Ok(v) => Ok(Some(
                Location::try_from(v).map_err(|_| GeoIpError::InternalError)?,
            )),
            Err(e) => match e {
                MaxMindDBError::AddressNotFoundError(_) => Ok(None),
                _ => Err(GeoIpError::InternalError),
            },
        }
    }
}

impl<'a> TryFrom<&City<'a>> for Location {
    type Error = String;

    fn try_from(city: &City) -> Result<Self, Self::Error> {
        let data = city.country.as_ref().ok_or_else(|| {
            warn!("No Country data found");
            "No Country data found"
        })?;
        let iso_alpha2 = String::from(data.iso_code.ok_or_else(|| {
            warn!("No iso alpha-2 code found in Country data {:#?}", data);
            "No iso alpha-2 code found in Country data"
        })?);
        let iso_codes = CountryCode::for_alpha2(&iso_alpha2).map_err(|e| {
            let message = format!(
                "Fail to get iso codes from iso alpha-2 country code {}: {}",
                &iso_alpha2, e
            );
            warn!("{}", &message);
            message
        })?;

        Ok(Location {
            iso_alpha2,
            iso_alpha3: String::from(iso_codes.alpha3()),
            name: String::from(iso_codes.name()),
            latitude: city.location.as_ref().and_then(|l| l.latitude),
            longitude: city.location.as_ref().and_then(|l| l.longitude),
        })
    }
}

impl ThreadsafeGeoIp {
    pub fn new() -> Self {
        ThreadsafeGeoIp(Arc::new(GeoIp::new()))
    }
}
