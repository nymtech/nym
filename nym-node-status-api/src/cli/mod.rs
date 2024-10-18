use clap::Parser;
use nym_bin_common::bin_info;
use reqwest::Url;
use std::{sync::OnceLock, time::Duration};

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the Nym API.
    #[clap(short, long, env = "NYM_NODE_STATUS_API_ENV_FILE")]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Network name for the network to which we're connecting.
    #[clap(long, env = "NYM_NODE_STATUS_API_NETWORK_NAME")]
    pub(crate) network_name: String,

    /// Explorer api url.
    #[clap(short, long, env = "NYM_NODE_STATUS_API_EXPLORER_API")]
    pub(crate) explorer_api: String,

    /// Nym api url.
    #[clap(short, long, env = "NYM_NODE_STATUS_API_NYM_API")]
    pub(crate) nym_api: String,

    /// TTL for the http cache.
    #[clap(
        long,
        default_value_t = 30,
        env = "NYM_NODE_STATUS_API_NYM_HTTP_CACHE_TTL"
    )]
    pub(crate) nym_http_cache_ttl: u64,

    /// HTTP port on which to run node status api.
    #[clap(long, default_value_t = 8000, env = "NYM_NODE_STATUS_API_HTTP_PORT")]
    pub(crate) http_port: u16,

    /// Nyxd address.
    #[clap(long, env = "NYM_NODE_STATUS_API_NYXD_ADDR")]
    pub(crate) nyxd_addr: Url,

    /// Nym api client timeout.
    #[clap(
        long,
        default_value = "15",
        env = "NYM_NODE_STATUS_API_NYM_API_CLIENT_TIMEOUT"
    )]
    #[arg(value_parser = parse_duration)]
    pub(crate) nym_api_client_timeout: Duration,

    /// Explorer api client timeout.
    #[clap(
        long,
        default_value = "15",
        env = "NYM_NODE_STATUS_API_EXPLORER_CLIENT_TIMEOUT"
    )]
    #[arg(value_parser = parse_duration)]
    pub(crate) explorer_client_timeout: Duration,

    /// Connection url for the database.
    #[clap(long, env = "NYM_NODE_STATUS_API_CONNECTION_URL")]
    pub(crate) connection_url: String,
}

fn parse_duration(arg: &str) -> Result<std::time::Duration, std::num::ParseIntError> {
    let seconds = arg.parse()?;
    Ok(std::time::Duration::from_secs(seconds))
}
