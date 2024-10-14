use anyhow::anyhow;
use reqwest::Url;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Config {
    #[serde(default = "Config::default_http_cache_seconds")]
    nym_http_cache_ttl: u64,
    #[serde(default = "Config::default_http_port")]
    http_port: u16,
    #[serde(rename = "nyxd")]
    nyxd_addr: Url,
    #[serde(default = "Config::default_client_timeout")]
    #[serde(deserialize_with = "parse_duration")]
    nym_api_client_timeout: Duration,
    #[serde(default = "Config::default_client_timeout")]
    #[serde(deserialize_with = "parse_duration")]
    explorer_client_timeout: Duration,
}

impl Config {
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        envy::from_env::<Self>().map_err(|e| {
            tracing::error!("Failed to load config from env: {e}");
            anyhow::Error::from(e)
        })
    }

    fn default_client_timeout() -> Duration {
        Duration::from_secs(15)
    }

    fn default_http_port() -> u16 {
        8000
    }

    fn default_http_cache_seconds() -> u64 {
        30
    }

    pub(crate) fn nym_http_cache_ttl(&self) -> u64 {
        self.nym_http_cache_ttl
    }

    pub(crate) fn http_port(&self) -> u16 {
        self.http_port
    }

    pub(crate) fn nyxd_addr(&self) -> &Url {
        &self.nyxd_addr
    }

    pub(crate) fn nym_api_client_timeout(&self) -> Duration {
        self.nym_api_client_timeout.to_owned()
    }

    pub(crate) fn nym_explorer_client_timeout(&self) -> Duration {
        self.explorer_client_timeout.to_owned()
    }
}

fn parse_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let secs: u64 = s.parse().map_err(serde::de::Error::custom)?;
    Ok(Duration::from_secs(secs))
}

pub(super) fn read_env_var(env_var: &str) -> anyhow::Result<String> {
    std::env::var(env_var)
        .map_err(|_| anyhow!("You need to set {}", env_var))
        .map(|value| {
            tracing::trace!("{}={}", env_var, value);
            value
        })
}
