use cosmwasm_std::{Addr, Coin};
use ipinfo::{IpInfo, IpInfoConfig};
use serde::{Deserialize, Serialize};

pub(crate) struct IpInfoClient {
    client: IpInfo,
}

impl IpInfoClient {
    pub(crate) fn new(token: impl Into<String>) -> anyhow::Result<Self> {
        let config = IpInfoConfig {
            token: Some(token.into()),
            ..Default::default()
        };
        let client = IpInfo::new(config)?;

        Ok(Self { client })
    }

    pub(crate) async fn locate_ip(&mut self, ip: impl AsRef<str>) -> anyhow::Result<Location> {
        self.client
            .lookup(ip.as_ref())
            .await
            .map(|details| Location {
                two_letter_iso_country_code: details.country,
            })
            .map_err(|err| {
                if matches!(err.kind(), ipinfo::IpErrorKind::RateLimitExceededError) {
                    tracing::error!("Rate limit exceeded: {}", err);
                }
                anyhow::Error::from(err)
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
