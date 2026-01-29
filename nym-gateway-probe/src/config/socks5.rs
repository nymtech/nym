use clap::Args;

use crate::common::socks5_test::JsonRpcClient;

#[derive(Args)]
pub struct Socks5Args {
    #[arg(long, value_delimiter = ';')]
    pub socks5_json_rpc_url_list: Vec<String>,

    #[arg(long, default_value_t = 30)]
    pub mixnet_client_timeout_sec: u64,

    #[arg(long, default_value_t = 10)]
    pub test_count: u64,

    /// stops socks5 test early after this many failed attempts
    #[arg(long, default_value_t = 3)]
    pub failure_count_cutoff: usize,
}

impl Socks5Args {
    pub async fn validate_socks5_endpoints(&self) -> anyhow::Result<()> {
        let client = JsonRpcClient::new(
            self.mixnet_client_timeout_sec,
            None,
            self.socks5_json_rpc_url_list.clone(),
        )?;
        client.ensure_endpoint_works().await?;

        Ok(())
    }
}
