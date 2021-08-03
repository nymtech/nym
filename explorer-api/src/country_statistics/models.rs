use serde::Deserialize;

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
