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
    command: Option<Commands>,
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

//async fn query(provider: mixnet::Recipient, request: Request) {
//    //let request_binary_info = ControlRequest::BinaryInfo;
//    //let full_request_binary_info: Request =
//        //Request::new_control(ProviderInterfaceVersion::new_current(), request);
//
//    // TODO: currently we HAVE TO use surbs unfortunately
//    println!("Sending 'BinaryInfo' request...");
//    client
//        .send_bytes(
//            provider,
//            request.into_bytes(),
//            mixnet::IncludedSurbs::new(10),
//        )
//        .await;
//    let response = wait_for_control_response(&mut client).await;
//    let binary_info = match response {
//        ControlResponse::BinaryInfo(binary_info) => binary_info,
//        _ => panic!("received wrong response type!"),
//    };
//    //println!("response to 'BinaryInfo' request: {response:#?}");
//    println!("{:#?}", *binary_info);
//}

//fn new_bin_info_request() -> Vec<u8> {
//    let request_binary_info = ControlRequest::BinaryInfo;
//    Request::<Empty>::new_control(ProviderInterfaceVersion::new_current(), request_binary_info)
//        .into_bytes()
//}

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

    //match args.command {
    //    Some(Commands::BinaryInfo) => todo!(),
    //    Some(Commands::SupportedRequestVersions) => todo!(),
    //    Some(Commands::OpenProxy) => todo!(),
    //    Some(Commands::All) => todo!(),
    //    None => todo!(),
    //}

    //println!("Sending 'BinaryInfo' request...");
    //client
    //    .send_bytes(provider, new_bin_info_request(), IncludedSurbs::new(10))
    //    .await;

    //
    // BinaryInfo
    //

    let request_binary_info = ControlRequest::BinaryInfo;
    let full_request_binary_info: Request =
        Request::new_control(ProviderInterfaceVersion::new_current(), request_binary_info);

    // TODO: currently we HAVE TO use surbs unfortunately
    println!("Sending 'BinaryInfo' request...");
    client
        .send_bytes(
            provider,
            full_request_binary_info.into_bytes(),
            IncludedSurbs::new(10),
        )
        .await;
    let response = wait_for_control_response(&mut client).await;
    let binary_info = match response {
        ControlResponse::BinaryInfo(binary_info) => binary_info,
        _ => panic!("received wrong response type!"),
    };
    //println!("response to 'BinaryInfo' request: {response:#?}");
    println!("{:#?}", *binary_info);

    //
    // SupportedRequestVersions
    //

    let request_versions = ControlRequest::SupportedRequestVersions;
    let full_request_versions: Request =
        Request::new_control(ProviderInterfaceVersion::new_current(), request_versions);

    println!("Sending 'SupportedRequestVersions' request...");
    client
        .send_bytes(
            provider,
            full_request_versions.into_bytes(),
            IncludedSurbs::new(10),
        )
        .await;
    let response = wait_for_control_response(&mut client).await;
    println!("response to 'SupportedRequestVersions' request: {response:#?}");

    //
    // OpenProxy
    //

    let request_open_proxy = Socks5Request::new_query(
        Socks5ProtocolVersion::new_current(),
        QueryRequest::OpenProxy,
    );
    let open_proxy_request =
        Request::new_provider_data(ProviderInterfaceVersion::new_current(), request_open_proxy);
    client
        .send_bytes(
            provider,
            open_proxy_request.into_bytes(),
            IncludedSurbs::new(10),
        )
        .await;
    let response = wait_for_socks5_response(&mut client).await;
    let open_proxy = match response.content {
        Socks5ResponseContent::Query(query) => query,
        _ => panic!("received wrong response type!"),
    };
    println!("response to 'OpenProxy' request: {open_proxy:#?}");

    println!("disconnecting");
    client.disconnect().await;

    Ok(())
}
