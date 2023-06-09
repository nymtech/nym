use clap::{Parser, Subcommand};
use nym_sdk::mixnet::{self, IncludedSurbs};
use nym_service_providers_common::interface::{
    ControlRequest, ControlResponse, ProviderInterfaceVersion, Request, Response, ResponseContent,
};
use nym_socks5_requests::{
    QueryRequest, QueryResponse, Socks5ProtocolVersion, Socks5Request, Socks5Response,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    config_env_file: Option<std::path::PathBuf>,

    #[arg(short, long)]
    provider: mixnet::Recipient,

    #[arg(short, long)]
    gateway: Option<mixnet::NodeIdentity>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Binary information
    BinaryInfo,

    /// Supported request versions
    SupportedRequestVersions,

    /// Check if the network requester is acting a an open proxy
    OpenProxy,

    /// Query all available properties
    All,
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
        let next = client.wait_for_messages().await.unwrap();
        if !next.is_empty() {
            return parse_control_response(next);
        }
    }
}

async fn wait_for_socks5_response(client: &mut mixnet::MixnetClient) -> Socks5Response {
    loop {
        let next = client.wait_for_messages().await.unwrap();
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

fn new_supported_request_versions_request() -> Vec<u8> {
    let request_versions = ControlRequest::SupportedRequestVersions;
    let request: Request =
        Request::new_control(ProviderInterfaceVersion::new_current(), request_versions);
    request.into_bytes()
}

fn new_open_proxy_request() -> Vec<u8> {
    let request_open_proxy = Socks5Request::new_query(
        Socks5ProtocolVersion::new_current(),
        QueryRequest::OpenProxy,
    );
    let open_proxy_request =
        Request::new_provider_data(ProviderInterfaceVersion::new_current(), request_open_proxy);
    open_proxy_request.into_bytes()
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

    async fn query_supported_request_versions(&mut self) -> ControlResponse {
        self.client
            .send_bytes(
                self.provider,
                new_supported_request_versions_request(),
                IncludedSurbs::new(10),
            )
            .await;
        wait_for_control_response(&mut self.client).await
    }

    async fn query_open_proxy(&mut self) -> QueryResponse {
        self.client
            .send_bytes(
                self.provider,
                new_open_proxy_request(),
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
}

#[derive(Debug)]
enum ClientResponse {
    Control(ControlResponse),
    Query(QueryResponse),
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // WIP(JON): should we expose this through the sdk?
    //if logging {
    //nym_bin_common::logging::setup_logging();
    //}

    let args = Cli::parse();

    // WIP(JON): should we expose this through the sdk?
    nym_network_defaults::setup_env(args.config_env_file.as_ref());

    println!("Connecting to mixnet...");
    let mut client = QueryClient::new(args.provider, args.gateway).await;

    println!("Sending request...");
    let resp: ClientResponse = match args.command {
        Commands::BinaryInfo => client.query_bin_info().await.into(),
        Commands::SupportedRequestVersions => {
            client.query_supported_request_versions().await.into()
        }
        Commands::OpenProxy => client.query_open_proxy().await.into(),
        Commands::All => todo!(),
    };
    println!("{resp:#?}");

    println!("Disconnecting...");
    client.client.disconnect().await;

    Ok(())
}
