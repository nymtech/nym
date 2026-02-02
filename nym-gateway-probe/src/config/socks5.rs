use clap::Args;

use crate::common::socks5_test::JsonRpcClient;

const DEFAULT_RPC_ENDPOINT: &str = "https://cloudflare-eth.com";

#[derive(Args, Debug)]
pub struct Socks5Args {
    #[arg(long, hide = true, value_delimiter = ';', default_value = DEFAULT_RPC_ENDPOINT)]
    pub socks5_json_rpc_url_list: Vec<String>,

    #[arg(long, hide = true, default_value_t = 30)]
    pub mixnet_client_timeout_sec: u64,

    #[arg(long, hide = true, default_value_t = 10)]
    pub test_count: u64,

    /// stops socks5 test early after this many failed attempts
    #[arg(long, hide = true, default_value_t = 3)]
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
