use ipinfo::{IpDetails, IpInfo, IpInfoConfig};
use std::time::Duration;

pub(crate) struct IpInfoClient {
    client: IpInfo,
}

impl IpInfoClient {
    pub(crate) fn new(token: impl Into<String>, timeout: Duration) -> anyhow::Result<Self> {
        let config = IpInfoConfig {
            token: Some(token.into()),
            timeout,
            ..Default::default()
        };
        let client = IpInfo::new(config)?;

        Ok(Self { client })
    }

    pub(crate) async fn lookup(&mut self, ip: &str) -> anyhow::Result<IpDetails> {
        self.client.lookup(ip).await.map_err(From::from)
    }
}
