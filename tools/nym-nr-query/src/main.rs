use std::fmt;

use clap::{Parser, ValueEnum};
use nym_bin_common::output_format::OutputFormat;
use nym_sdk::mixnet::{self, IncludedSurbs};
use nym_service_providers_common::interface::{
    ControlRequest, ControlResponse, ProviderInterfaceVersion, Request, Response, ResponseContent,
};
use nym_socks5_requests::{
    QueryRequest, QueryResponse, Socks5ProtocolVersion, Socks5Request, Socks5Response,
};
use serde::Serialize;
use tokio::time::{timeout, Duration};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    config_env_file: Option<std::path::PathBuf>,

    #[arg(short, long)]
    debug: bool,

    #[arg(short, long)]
    provider: mixnet::Recipient,

    #[arg(short, long)]
    gateway: Option<mixnet::NodeIdentity>,

    #[arg(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,

    #[arg(value_enum, default_value_t = Commands::Ping)]
    command: Commands,
}

#[derive(Clone, ValueEnum, PartialEq, Eq)]
enum Commands {
    /// Binary information
    BinaryInfo,

    /// Supported request versions
    SupportedRequestVersions,

    /// Check if the network requester is acting a an open proxy
    OpenProxy,

    /// Ping the network requester
    Ping,
}

fn parse_control_response(received: Vec<mixnet::ReconstructedMessage>) -> ControlResponse {
    assert_eq!(received.len(), 1);
    let response: Response = Response::try_from_bytes(&received[0].message).unwrap();
    match response.content {
        ResponseContent::Control(control) => control,
        ResponseContent::ProviderData(_) => {
            panic!("received provider data even though we sent control request!")
        }
    }
}

fn parse_socks5_response(received: Vec<mixnet::ReconstructedMessage>) -> Socks5Response {
    assert_eq!(received.len(), 1);
    let response: Response<Socks5Request> = Response::try_from_bytes(&received[0].message).unwrap();
    match response.content {
        ResponseContent::Control(control) => panic!("unexpected control response: {:?}", control),
        ResponseContent::ProviderData(data) => data,
    }
}

async fn wait_for_control_response(client: &mut mixnet::MixnetClient) -> ControlResponse {
    loop {
        let Ok(next) = timeout(RESPONSE_TIMEOUT, client.wait_for_messages()).await else {
            eprintln!("Timeout waiting for response");
            std::process::exit(1);
        };
        let next = next.unwrap();
        if !next.is_empty() {
            return parse_control_response(next);
        }
    }
}

async fn wait_for_socks5_response(client: &mut mixnet::MixnetClient) -> Socks5Response {
    loop {
        let Ok(next) = timeout(RESPONSE_TIMEOUT, client.wait_for_messages()).await else {
            eprintln!("Timeout waiting for response");
            std::process::exit(1);
        };
        let next = next.unwrap();
        if !next.is_empty() {
            return parse_socks5_response(next);
        }
    }
}

async fn connect_to_mixnet(gateway: Option<mixnet::NodeIdentity>) -> mixnet::MixnetClient {
    match gateway {
        Some(gateway) => mixnet::MixnetClientBuilder::new_ephemeral()
            .request_gateway(gateway.to_base58_string())
            .build()
            .await
            .expect("Failed to create mixnet client")
            .connect_to_mixnet()
            .await
            .expect("Failed to connect to the mixnet"),
        None => mixnet::MixnetClient::connect_new().await.unwrap(),
    }
}

fn new_bin_info_request() -> Request {
    let request_binary_info = ControlRequest::BinaryInfo;
    Request::new_control(ProviderInterfaceVersion::new_current(), request_binary_info)
}

fn new_supported_request_versions_request() -> Request {
    let request_versions = ControlRequest::SupportedRequestVersions;
    Request::new_control(ProviderInterfaceVersion::new_current(), request_versions)
}

fn new_open_proxy_request() -> Request<Socks5Request> {
    let request_open_proxy = Socks5Request::new_query(
        Socks5ProtocolVersion::new_current(),
        QueryRequest::OpenProxy,
    );
    Request::new_provider_data(ProviderInterfaceVersion::new_current(), request_open_proxy)
}

fn new_ping_request() -> Request {
    let request_ping = ControlRequest::Health;
    Request::new_control(ProviderInterfaceVersion::new_current(), request_ping)
}

struct QueryClient {
    pub client: mixnet::MixnetClient,
    pub provider: mixnet::Recipient,
}

impl QueryClient {
    async fn new(provider: mixnet::Recipient, gateway: Option<mixnet::NodeIdentity>) -> Self {
        let client = connect_to_mixnet(gateway).await;
        Self { client, provider }
    }

    async fn query_bin_info(&mut self) -> ControlResponse {
        self.client
            .send_bytes(
                self.provider,
                new_bin_info_request().into_bytes(),
                IncludedSurbs::new(10),
            )
            .await;
        wait_for_control_response(&mut self.client).await
    }

    async fn query_supported_versions(&mut self) -> ControlResponse {
        self.client
            .send_bytes(
                self.provider,
                new_supported_request_versions_request().into_bytes(),
                IncludedSurbs::new(10),
            )
            .await;
        wait_for_control_response(&mut self.client).await
    }

    async fn query_open_proxy(&mut self) -> QueryResponse {
        self.client
            .send_bytes(
                self.provider,
                new_open_proxy_request().into_bytes(),
                IncludedSurbs::new(10),
            )
            .await;
        let response = wait_for_socks5_response(&mut self.client).await;
        response
            .content
            .as_query()
            .expect("Unexpected response type!")
            .clone()
    }

    async fn ping(&mut self) -> PingResponse {
        let now = std::time::Instant::now();
        self.client
            .send_bytes(
                self.provider,
                new_ping_request().into_bytes(),
                IncludedSurbs::new(10),
            )
            .await;
        let response = wait_for_control_response(&mut self.client).await;
        assert!(matches!(response, ControlResponse::Health));
        let elapsed = now.elapsed();
        PingResponse {
            provider: self.provider.to_string(),
            ping_ms: elapsed.as_millis(),
        }
    }
}

#[derive(Debug, Serialize)]
struct PingResponse {
    provider: String,
    ping_ms: u128,
}

impl fmt::Display for PingResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:  time={} ms", self.provider, self.ping_ms)
    }
}

#[derive(Debug, Serialize)]
enum ClientResponse {
    Control(ControlResponse),
    Query(QueryResponse),
    Ping(PingResponse),
}

impl fmt::Display for ClientResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientResponse::Control(control) => write!(f, "{:#?}", control),
            ClientResponse::Query(query) => write!(f, "{:#?}", query),
            ClientResponse::Ping(ping) => write!(f, "{}", ping),
        }
    }
}

impl From<ControlResponse> for ClientResponse {
    fn from(response: ControlResponse) -> Self {
        ClientResponse::Control(response)
    }
}

impl From<QueryResponse> for ClientResponse {
    fn from(response: QueryResponse) -> Self {
        ClientResponse::Query(response)
    }
}

impl From<PingResponse> for ClientResponse {
    fn from(response: PingResponse) -> Self {
        ClientResponse::Ping(response)
    }
}

fn text_print(input: &str, output: &OutputFormat) {
    if output.is_text() {
        println!("{input}");
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    if args.debug {
        nym_bin_common::logging::setup_logging();
    }

    nym_network_defaults::setup_env(args.config_env_file.as_ref());

    text_print("Registering with gateway...", &args.output);
    let mut client = QueryClient::new(args.provider, args.gateway).await;

    text_print("Sending request...", &args.output);
    if args.command == Commands::Ping {
        for _ in 0..4 {
            let resp: ClientResponse = client.ping().await.into();
            println!("{}", args.output.format(&resp));
        }

    } else {
        let resp: ClientResponse = match args.command {
            Commands::BinaryInfo => client.query_bin_info().await.into(),
            Commands::SupportedRequestVersions => client.query_supported_versions().await.into(),
            Commands::OpenProxy => client.query_open_proxy().await.into(),
            Commands::Ping => unreachable!(),
        };
        println!("{}", args.output.format(&resp));
    }

    text_print("Disconnecting...", &args.output);
    client.client.disconnect().await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
