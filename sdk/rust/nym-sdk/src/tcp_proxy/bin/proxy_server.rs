use anyhow::Result;
use clap::Parser;
use nym_sdk::tcp_proxy;

#[derive(Parser, Debug)]
struct Args {
    /// Upstream address of the server process we want to proxy traffic to e.g. 127.0.0.1:9067
    #[clap(short, long)]
    upstream_tcp_address: String,

    /// Config directory
    #[clap(short, long, default_value = "/tmp/nym-tcp-proxy-server")]
    config_dir: String,

    /// Optional env filepath - if none is supplied then the proxy defaults to using mainnet else just use a path to one of the supplied files in envs/ e.g. ./envs/sandbox.env
    #[clap(short, long)]
    env_path: Option<String>
}

#[tokio::main]
async fn main() -> Result<()> {
    nym_bin_common::logging::setup_logging();

    let args = Args::parse();

    let home_dir = dirs::home_dir().expect("Unable to get home directory");
    let conf_path = format!("{}{}", home_dir.display(), args.config_dir);

    let mut proxy_server = tcp_proxy::NymProxyServer::new(&args.upstream_tcp_address, &conf_path, args.env_path.clone()).await?;

    proxy_server.run_with_shutdown().await
}
