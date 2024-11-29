use cosmwasm_std::{Addr, Coin};
use serde::{Deserialize, Serialize};

pub(crate) struct IpInfoClient {
    client: reqwest::Client,
    token: String,
}

impl IpInfoClient {
    pub(crate) fn new(token: impl Into<String>) -> anyhow::Result<Self> {
        let client = reqwest::Client::new();
        let token = token.into();

        Ok(Self { client, token })
    }

    pub(crate) async fn locate_ip(&self, ip: impl AsRef<str>) -> anyhow::Result<Location> {
        let url = format!(
            "https://ipinfo.io/{}/country?token={}",
            ip.as_ref(),
            &self.token
        );
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
        let response_text = response.text().await?;

        Ok(Location {
            two_letter_iso_country_code: response_text,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NodeGeoData {
    pub(crate) identity_key: String,
    pub(crate) owner: Addr,
    pub(crate) pledge_amount: Coin,
    pub(crate) location: Location,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct Location {
    pub(crate) two_letter_iso_country_code: String,
}

impl Location {
    pub(crate) fn empty() -> Self {
        Self {
            two_letter_iso_country_code: String::new(),
        }
    }
}
