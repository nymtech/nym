use clap::{Parser, Subcommand};
use nym_sdk::mixnet::{self, IncludedSurbs};
use nym_service_providers_common::interface::{
    ControlRequest, ControlResponse, Empty, ProviderInterfaceVersion, Request, Response,
    ResponseContent,
};
use nym_socks5_requests::{
    QueryRequest, Socks5ProtocolVersion, Socks5Request, Socks5Response, Socks5ResponseContent,
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

async fn wait_for_control_response(client: &mut mixnet::MixnetClient) -> ControlResponse {
    loop {
        let next = client.wait_for_messages().await.unwrap();
        if !next.is_empty() {
            return parse_control_response(next);
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

fn new_bin_info_request() -> Vec<u8> {
    let request_binary_info = ControlRequest::BinaryInfo;
    let request: Request =
        Request::new_control(ProviderInterfaceVersion::new_current(), request_binary_info);
    request.into_bytes()
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // WIP(JON): should we expose this through the sdk?
    //if logging {
    //nym_bin_common::logging::setup_logging();
    //}

    let args = Cli::parse();

    // WIP(JON): should we expose this through the sdk?
    nym_network_defaults::setup_env(args.config_env_file.as_ref());

    let provider = args.provider;
    let mut client = connect_to_mixnet(args.gateway).await;

    match args.command {
        Commands::BinaryInfo => {
            println!("Sending 'BinaryInfo' request...");
            client
                .send_bytes(provider, new_bin_info_request(), IncludedSurbs::new(10))
                .await;
            let response = wait_for_control_response(&mut client).await;
            let binary_info = response.binary_info().expect("Unexpected response type!");
            println!("{:#?}", *binary_info);
        }
        Commands::SupportedRequestVersions => {
            println!("Sending 'SupportedRequestVersions' request...");
            client
                .send_bytes(
                    provider,
                    new_supported_request_versions_request(),
                    IncludedSurbs::new(10),
                )
                .await;
            let response = wait_for_control_response(&mut client).await;
            let supported_request_versions = response
                .supported_request_versions()
                .expect("Unexpected response type!");
            println!("{supported_request_versions:#?}");
        }
        Commands::OpenProxy => {
            client
                .send_bytes(provider, new_open_proxy_request(), IncludedSurbs::new(10))
                .await;
            let response = wait_for_socks5_response(&mut client).await;
            let open_proxy = response
                .content
                .as_query()
                .expect("Unexpected response type!");
            println!("{open_proxy:#?}");
        }
        Commands::All => todo!(),
    }

    //println!("disconnecting");
    client.disconnect().await;

    Ok(())
}
