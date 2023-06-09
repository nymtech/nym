use clap::Parser;
use nym_sdk::mixnet::{self};
use nym_service_providers_common::interface::{
    ControlRequest, ControlResponse, ProviderInterfaceVersion, Request, Response, ResponseContent,
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // WIP(JON): should we expose this through the sdk?
    //if logging {
    //nym_bin_common::logging::setup_logging();
    //}

    let args = Cli::parse();

    // WIP(JON): should we expose this through the sdk?
    nym_network_defaults::setup_env(args.config_env_file.as_ref());

    // WIP(JON): pass in gateway
    let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    let provider = args.provider;

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
            mixnet::IncludedSurbs::new(10),
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
            mixnet::IncludedSurbs::new(10),
        )
        .await;
    let response = wait_for_control_response(&mut client).await;
    println!("response to 'SupportedRequestVersions' request: {response:#?}");

    Ok(())
}
