use clap::Args;

use crate::common::socks5_test::JsonRpcClient;

#[derive(Args, Clone, Debug)]
pub struct Socks5Args {
    #[arg(long, hide = true, default_values_t = Socks5Args::default().socks5_json_rpc_url_list, value_delimiter = ';')]
    pub socks5_json_rpc_url_list: Vec<String>,

    #[arg(long, hide = true, default_value_t = Socks5Args::default().mixnet_client_timeout_sec)]
    pub mixnet_client_timeout_sec: u64,

    #[arg(long, hide = true, default_value_t = Socks5Args::default().test_count)]
    pub test_count: u64,

    /// stops socks5 test early after this many failed attempts
    #[arg(long, hide = true, default_value_t = Socks5Args::default().failure_count_cutoff)]
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

impl Default for Socks5Args {
    fn default() -> Self {
        Self {
            socks5_json_rpc_url_list: vec![
                "https://cloudflare-eth.com".to_string(),
                "https://ethereum-rpc.publicnode.com".to_string(),
            ],
            mixnet_client_timeout_sec: 30,
            test_count: 10,
            failure_count_cutoff: 3,
        }
    }
}
