use anyhow::Result;
use clap::Parser;
use nym_sdk::tcp_proxy;
use nym_sphinx_addressing::Recipient;

#[derive(Parser, Debug)]
struct Args {
    /// Send timeout in seconds
    #[clap(long, default_value_t = 30)]
    close_timeout: u64,

    /// Nym address of the NymProxyServer e.g. EjYsntVxxBJrcRugiX5VnbKMbg7gyBGSp9SLt7RgeVFV.EzRtVdHCHoP2ho3DJgKMisMQ3zHkqMtAFAW4pxsq7Y2a@Hs463Wh5LtWZU@NyAmt4trcCbNVsuUhry1wpEXpVnAAfn
    #[clap(short, long)]
    server_address: String,

    /// Listen address
    #[clap(long, default_value = "127.0.0.1")]
    listen_address: String,

    /// Listen port
    #[clap(long, default_value = "8080")]
    listen_port: String,

    /// Optional env filepath - if none is supplied then the proxy defaults to using mainnet else just use a path to one of the supplied files in envs/ e.g. ./envs/sandbox.env
    #[clap(short, long)]
    env_path: Option<String>,

    /// How many clients to have running in reserve for quick access by incoming connections
    #[clap(long, default_value_t = 2)]
    client_pool_reserve: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    nym_bin_common::logging::setup_tracing_logger();
    let args = Args::parse();

    let nym_addr: Recipient =
        Recipient::try_from_base58_string(&args.server_address).expect("Invalid server address");

    let proxy_client = tcp_proxy::NymProxyClient::new(
        nym_addr,
        &args.listen_address,
        &args.listen_port,
        args.close_timeout,
        args.env_path.clone(),
        args.client_pool_reserve,
    )
    .await?;

    proxy_client.run().await.unwrap();

    Ok(())
}
