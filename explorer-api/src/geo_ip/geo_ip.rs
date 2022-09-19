use isocountry::CountryCode;
use log::warn;
use maxminddb::{geoip2::Country, MaxMindDBError, Reader};
use std::{net::IpAddr, str::FromStr, sync::Arc};

const GEOIP_DB_PATH: &str = "./src/geo_ip/GeoLite2-Country.mmdb";

#[derive(Debug)]
pub enum GeoIpError {
    NoValidIP,
    InternalError,
}

// The current State implementation does not allow to fail on state
// creation, ie. returning Result<>. To avoid to use unwrap family,
// as a workaround, wrap the state inside an Option<>
// If Reader::open_readfile fails for some reason db will will be set to None
// and an error will be logged.
pub(crate) struct GeoIp {
    pub(crate) db: Option<Reader<Vec<u8>>>,
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
}

impl GeoIp {
    pub fn new() -> Self {
        let reader = Reader::open_readfile(GEOIP_DB_PATH)
            .map_err(|e| {
                error!(
                    "Fail to open GeoLite2 database file {}: {}",
                    GEOIP_DB_PATH, e
                );
            })
            .ok();
        GeoIp { db: reader }
    }

    pub fn query(&self, address: &str) -> Result<Option<Location>, GeoIpError> {
        let ip: IpAddr = FromStr::from_str(address).map_err(|e| {
            error!("Fail to create IpAddr from {}: {}", &address, e);
            GeoIpError::NoValidIP
        })?;
        let result = self
            .db
            .as_ref()
            .ok_or_else(|| {
                error!("No registered GeoIP database");
                GeoIpError::InternalError
            })?
            .lookup::<Country>(ip);
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

impl<'a> TryFrom<&Country<'a>> for Location {
    type Error = String;

    fn try_from(country: &Country) -> Result<Self, Self::Error> {
        let data = country.country.as_ref().ok_or_else(|| {
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
        })
    }
}

impl ThreadsafeGeoIp {
    pub fn new() -> Self {
        ThreadsafeGeoIp(Arc::new(GeoIp::new()))
    }
}
