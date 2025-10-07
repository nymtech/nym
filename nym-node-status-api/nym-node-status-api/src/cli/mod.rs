use crate::ticketbook_manager::TicketbookManagerConfig;
use clap::Parser;
use nym_bin_common::bin_info;
use nym_credential_proxy_lib::shared_state::ecash_state::TicketType;
use reqwest::Url;
use std::str::FromStr;
use std::{sync::OnceLock, time::Duration};

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Debug, Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the binary.
    /// Useful in local testing setups against networks different from mainnet
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Network name for the network to which we're connecting.
    #[clap(long, env = "NETWORK_NAME")]
    pub(crate) network_name: String,

    /// Nym api url.
    #[clap(short, long, env = "NYM_API")]
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
    #[clap(long, env = "NYXD")]
    pub(crate) nyxd_addr: Url,

    /// Nym api client timeout.
    #[clap(long, default_value = "15", env = "NYM_API_CLIENT_TIMEOUT")]
    #[arg(value_parser = parse_duration_std)]
    pub(crate) nym_api_client_timeout: Duration,

    /// Connection url for the database.
    #[clap(long, env = "DATABASE_URL")]
    pub(crate) database_url: String,

    #[clap(long, default_value = "5", env = "SQLX_BUSY_TIMEOUT_S")]
    #[arg(value_parser = parse_duration_std)]
    pub(crate) sqlx_busy_timeout_s: Duration,

    #[clap(
        long,
        default_value = "300",
        env = "NODE_STATUS_API_MONITOR_REFRESH_INTERVAL"
    )]
    #[arg(value_parser = parse_duration_std)]
    pub(crate) monitor_refresh_interval: Duration,

    #[clap(
        long,
        default_value = "300",
        env = "NODE_STATUS_API_TESTRUN_REFRESH_INTERVAL"
    )]
    #[arg(value_parser = parse_duration_std)]
    pub(crate) testruns_refresh_interval: Duration,

    #[clap(long, default_value = "86400", env = "NODE_STATUS_API_GEODATA_TTL")]
    #[arg(value_parser = parse_duration_std)]
    pub(crate) geodata_ttl: Duration,

    #[clap(env = "NODE_STATUS_API_AGENT_KEY_LIST")]
    #[arg(value_delimiter = ',')]
    pub(crate) agent_key_list: Vec<String>,

    #[clap(long, default_value = "120s", env = "AGENT_REQUEST_FRESHNESS")]
    #[arg(value_parser = parse_duration_humantime)]
    pub(crate) agent_request_freshness: time::Duration,

    #[clap(
        long,
        default_value_t = 10,
        env = "NYM_NODE_STATUS_API_PACKET_STATS_MAX_CONCURRENT_TASKS"
    )]
    pub(crate) packet_stats_max_concurrent_tasks: usize,

    /// https://github.com/ipinfo/rust
    #[clap(long, env = "IPINFO_API_TOKEN")]
    pub(crate) ipinfo_api_token: String,

    #[clap(
        long,
        default_value_t = 40,
        env = "NYM_NODE_STATUS_API_MAX_AGENT_COUNT"
    )]
    pub(crate) max_agent_count: i64,

    #[clap(flatten)]
    pub(crate) ticketbook: TicketbookArgs,
}

#[derive(Debug, Parser)]
pub(crate) struct TicketbookArgs {
    /// Specifies the mnemonic authorised for making deposits for the ticketbooks
    #[clap(long, env = "NYM_NODE_STATUS_API_MNEMONIC")]
    pub mnemonic: bip39::Mnemonic,

    /// Specify the maximum number of deposits the node status api can make in a single transaction.
    /// Note that each deposit batch is followed by the same number of sequential signing requests
    /// (default: 5)
    #[clap(
        long,
        env = "NYM_NODE_STATUS_API_MAX_CONCURRENT_DEPOSITS",
        default_value_t = 5
    )]
    pub(crate) max_concurrent_deposits: usize,

    /// Specify the size of the tickets buffer the node status api should have available at any time
    /// for each ticket type.
    /// (default: 50)
    #[clap(long, env = "NYM_NODE_STATUS_API_TICKETS_BUFFER", default_value_t = 50)]
    pub(crate) tickets_buffer_size: usize,

    /// Specify interval at which the node status api should check if it has sufficient number of tickets buffered.
    /// (default: 1 minute)
    #[clap(
        long,
        env = "NYM_NODE_STATUS_API_TICKETS_CHECK_INTERVAL",
        default_value = "1m",
        value_parser = humantime::parse_duration
    )]
    pub(crate) tickets_buffer_check_interval: Duration,

    /// Specify interval at which the node status api should check if signing quorum is available
    /// (default: 5 minutes)
    #[clap(
        long,
        env = "NYM_NODE_STATUS_API_QUORUM_CHECK_INTERVAL",
        default_value = "5m",
        value_parser = humantime::parse_duration
    )]
    pub(crate) quorum_check_interval: Duration,

    /// Specify types of tickets to buffer
    /// (default: V1MixnetEntry, V1WireguardEntry, V1WireguardExit)
    #[clap(
        long,
        env = "NYM_NODE_STATUS_BUFFERED_TICKET_TYPES",
        default_values_t = [TicketType::V1MixnetEntry, TicketType::V1WireguardEntry, TicketType::V1WireguardExit]
    )]
    #[arg(value_delimiter = ',')]
    pub(crate) buffered_ticket_types: Vec<TicketType>,

    /// Identifier used for deriving keys embedded in the issued ticketbooks.
    /// It can be a random string, but make sure it has sufficient entropy
    #[clap(
        long,
        env = "NYM_NODE_STATUS_API_ECASH_CLIENT_IDENTIFIER_BS58",
        required = true
    )]
    pub(crate) ecash_client_identifier_bs58: ClientIdentifier,
}

#[derive(Debug, Clone)]
pub(crate) struct ClientIdentifier(pub(crate) Vec<u8>);

impl FromStr for ClientIdentifier {
    type Err = bs58::decode::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        bs58::decode(s).into_vec().map(ClientIdentifier)
    }
}

impl TicketbookArgs {
    pub(crate) fn to_manager_config(&self) -> TicketbookManagerConfig {
        TicketbookManagerConfig {
            check_interval: self.tickets_buffer_check_interval,
            tickets_buffer_size: self.tickets_buffer_size,
            max_concurrent_deposits: self.max_concurrent_deposits,
            buffered_ticket_types: self.buffered_ticket_types.clone(),
        }
    }
}

fn parse_duration_humantime(arg: &str) -> Result<time::Duration, anyhow::Error> {
    let std_duration = match humantime::parse_duration(arg) {
        Ok(duration) => duration,
        // assume old format (seconds) as a fallback
        Err(_) => parse_duration_std(arg)?,
    };

    Ok(time::Duration::seconds(std_duration.as_secs() as i64))
}

fn parse_duration_std(arg: &str) -> Result<std::time::Duration, std::num::ParseIntError> {
    let seconds = arg.parse()?;
    Ok(std::time::Duration::from_secs(seconds))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn humantime_should_work() {
        let should_parse = [("120s", 120), ("120", 120), ("0s", 0), ("0", 0)];

        for (raw, expected) in should_parse {
            if let Ok(parsed) = parse_duration_humantime(raw) {
                assert_eq!(parsed.whole_seconds(), expected);
            } else {
                panic!("Failed to parse {raw}")
            }
        }

        assert!(parse_duration_humantime("-15s").is_err());
    }
}
