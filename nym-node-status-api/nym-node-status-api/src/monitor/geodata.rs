use cosmwasm_std::{Addr, Coin};
use serde::{Deserialize, Deserializer, Serialize};

pub(crate) struct IpInfoClient {
    client: reqwest::Client,
    token: String,
}

impl IpInfoClient {
    pub(crate) fn new(token: impl Into<String>) -> Self {
        let client = reqwest::Client::new();
        let token = token.into();

        Self { client, token }
    }

    pub(crate) async fn locate_ip(&self, ip: impl AsRef<str>) -> anyhow::Result<Location> {
        let url = format!("https://ipinfo.io/{}?token={}", ip.as_ref(), &self.token);
        let response = self
            .client
            .get(url)
            .send()
            .await
            // map non 2xx responses to error
            .and_then(|res| res.error_for_status())
            .map_err(|err| {
                if matches!(err.status(), Some(reqwest::StatusCode::TOO_MANY_REQUESTS)) {
                    tracing::error!("ipinfo rate limit exceeded");
                }
                anyhow::Error::from(err)
            })?;

        // extracting text, then deserializing produces better error messages than response.json()
        let raw_response = response.text().await?;
        let response: LocationResponse =
            serde_json::from_str(&raw_response).inspect_err(|e| tracing::error!("{e}"))?;
        let location = response.into();

        Ok(location)
    }

    /// check DOESN'T consume bandwidth allowance
    pub(crate) async fn check_remaining_bandwidth(
        &self,
    ) -> anyhow::Result<ipinfo::MeResponseRequests> {
        let url = format!("https://ipinfo.io/me?token={}", &self.token);
        let response = self
            .client
            .get(url)
            .send()
            .await
            // map non 2xx responses to error
            .and_then(|res| res.error_for_status())
            .map_err(|err| {
                if matches!(err.status(), Some(reqwest::StatusCode::TOO_MANY_REQUESTS)) {
                    tracing::error!("ipinfo rate limit exceeded");
                }
                anyhow::Error::from(err)
            })?;
        let response: ipinfo::MeResponse = response.json().await?;

        Ok(response.requests)
    }
}

// TODO dz: are fields other than location used?
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExplorerPrettyBond {
    pub(crate) identity_key: String,
    pub(crate) owner: Addr,
    pub(crate) pledge_amount: Coin,
    pub(crate) location: Location,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct Location {
    pub(crate) two_letter_iso_country_code: String,
    #[serde(flatten)]
    pub(crate) location: Coordinates,
    pub(crate) ip_address: String,
    pub(crate) city: String,
    pub(crate) region: String,
    pub(crate) org: String,
    pub(crate) postal: String,
    pub(crate) timezone: String,
}

impl From<LocationResponse> for Location {
    fn from(value: LocationResponse) -> Self {
        Self {
            two_letter_iso_country_code: value.two_letter_iso_country_code,
            location: value.loc,
            ip_address: value.ip,
            city: value.city,
            region: value.region,
            org: value.org,
            postal: value.postal,
            timezone: value.timezone,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct LocationResponse {
    #[serde(rename = "country")]
    pub(crate) two_letter_iso_country_code: String,
    #[serde(deserialize_with = "deserialize_loc")]
    pub(crate) loc: Coordinates,
    // TODO dz consider making them optional?
    #[serde(default = "String::default")]
    pub(crate) ip: String,
    #[serde(default = "String::default")]
    pub(crate) city: String,
    #[serde(default = "String::default")]
    pub(crate) region: String,
    #[serde(default = "String::default")]
    pub(crate) org: String,
    #[serde(default = "String::default")]
    pub(crate) postal: String,
    #[serde(default = "String::default")]
    pub(crate) timezone: String,
}

fn deserialize_loc<'de, D>(deserializer: D) -> Result<Coordinates, D::Error>
where
    D: Deserializer<'de>,
{
    let loc_raw = String::deserialize(deserializer)?;
    match loc_raw.split_once(',') {
        Some((lat, long)) => Ok(Coordinates {
            latitude: lat.parse().map_err(serde::de::Error::custom)?,
            longitude: long.parse().map_err(serde::de::Error::custom)?,
        }),
        None => Err(serde::de::Error::custom("coordinates")),
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct Coordinates {
    pub(crate) latitude: f64,
    pub(crate) longitude: f64,
}

impl Location {
    pub(crate) fn empty() -> Self {
        Self::default()
    }
}

pub(crate) mod ipinfo {
    use super::*;

    // clippy doesn't understand it's used for typed deserialization
    #[allow(dead_code)]
    #[derive(Debug, Clone, Deserialize)]
    /// `/me` is undocumented in their developers page
    /// https://ipinfo.io/developers/responses
    /// but explained here
    /// https://community.ipinfo.io/t/easy-way-to-check-allowance-usage/5755/2
    pub(crate) struct MeResponse {
        token: String,
        pub(crate) requests: MeResponseRequests,
    }

    // clippy doesn't understand it's used for typed deserialization
    #[allow(dead_code)]
    #[derive(Debug, Clone, Deserialize)]
    pub(crate) struct MeResponseRequests {
        pub(crate) day: u64,
        pub(crate) month: u64,
        pub(crate) limit: u64,
        pub(crate) remaining: u64,
    }
}

#[cfg(test)]
mod api_regression {

    use super::*;
    use std::{env::var, sync::LazyLock};

    static IPINFO_TOKEN: LazyLock<Option<String>> = LazyLock::new(|| var("IPINFO_API_TOKEN").ok());
    static CI: LazyLock<Option<String>> = LazyLock::new(|| var("CI").ok());

    #[tokio::test]
    async fn should_parse_response() {
        if CI.is_none() {
            return;
        }
        if let Some(token) = &*IPINFO_TOKEN {
            let client = IpInfoClient::new(token);
            let my_ip = reqwest::get("https://api.ipify.org")
                .await
                .expect("Couldn't get own IP")
                .text()
                .await
                .unwrap();

            let location_result = client.locate_ip(my_ip).await;
            assert!(location_result.is_ok(), "Did ipinfo response change?");

            // This check fails almost every time on CI, possibly due to rate limiting?
            // It's not good to disable the check, but it's blocking CI as it stands now. Given
            // that we have the check above for locating the ip, we at least have a little
            // coverage.
            //client
            //    .check_remaining_bandwidth()
            //    .await
            //    .expect("Failed to check remaining bandwidth?");

            // when serialized, these fields should be present because they're exposed over API
            let location_result = location_result.unwrap();
            let json = serde_json::to_value(&location_result).unwrap();
            assert!(json.get("two_letter_iso_country_code").is_some());
            assert!(json.get("latitude").is_some());
            assert!(json.get("longitude").is_some());
        } else {
            panic!("IPINFO_API_TOKEN not set");
        }
    }
}
