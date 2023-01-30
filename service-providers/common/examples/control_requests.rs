// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client::client::config::{BaseConfig, Config, GatewayEndpointConfig};
use nym_client::client::{DirectClient, KeyManager, Recipient, ReconstructedMessage, SocketClient};
use rand::rngs::OsRng;
use service_providers_common::interface::{
    ControlRequest, ControlResponse, ProviderInterfaceVersion, Request, Response, ResponseContent,
};

fn parse_control_response(received: Vec<ReconstructedMessage>) -> ControlResponse {
    assert_eq!(received.len(), 1);
    let response: Response = Response::try_from_bytes(&received[0].message).unwrap();
    match response.content {
        ResponseContent::Control(control) => control,
        ResponseContent::ProviderData(_) => {
            panic!("received provider data even though we sent control request!")
        }
    }
}

async fn wait_for_control_response(client: &mut DirectClient) -> ControlResponse {
    loop {
        let next = client.wait_for_messages().await;
        if !next.is_empty() {
            return parse_control_response(next);
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // technically we don't need to start the entire client with all the subroutines,
    // but I needed an easy way of sending to and receiving from the mixnet
    // and that was the most straightforward way of achieving it
    let gateway_config = GatewayEndpointConfig {
        gateway_id: "E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM".to_string(),
        gateway_owner: r#"¯\_(ツ)_/¯"#.to_string(),
        gateway_listener: "ws://213.219.38.119:9000".to_string(),
    };

    let provider: Recipient = "AN8eLxYWFitCkMn92zim3PrPszxJZDYyFFKP7qnnAAew.8UAxL3LwQBis6WpM3GGXaqKGaVdnLCpGJWumHT6KNdTH@77TSuVU8d1oXKbPzjec2xh4i3Wj5WwUyy9Lr36sm8gZm".parse().unwrap();

    let client_config = Config::new("control-requests-example")
        .with_base(BaseConfig::with_disabled_credentials, true)
        .with_base(BaseConfig::with_disabled_cover_traffic, true)
        .with_base(BaseConfig::with_gateway_endpoint, gateway_config)
        .with_disabled_socket(true);

    let mut rng = OsRng;
    let key_manager = KeyManager::new(&mut rng);

    let mut client = SocketClient::new_with_keys(client_config, key_manager)
        .start_direct()
        .await?;

    // generic service provider request, so we don't even need to care it's to the socks5 provider
    let request_health = ControlRequest::Health;
    let request_binary_info = ControlRequest::BinaryInfo;
    let request_versions = ControlRequest::SupportedRequestVersions;

    let full_request_health: Request =
        Request::new_control(ProviderInterfaceVersion::new_current(), request_health);
    let full_request_binary_info: Request =
        Request::new_control(ProviderInterfaceVersion::new_current(), request_binary_info);
    let full_request_versions: Request =
        Request::new_control(ProviderInterfaceVersion::new_current(), request_versions);

    // TODO: currently we HAVE TO use surbs unfortunately
    client
        .send_anonymous_message(provider, full_request_health.into_bytes(), 10)
        .await;
    let response = wait_for_control_response(&mut client).await;
    println!("response to 'Health' request: {response:#?}");

    client
        .send_anonymous_message(provider, full_request_binary_info.into_bytes(), 10)
        .await;
    let response = wait_for_control_response(&mut client).await;
    println!("response to 'BinaryInfo' request: {response:#?}");

    client
        .send_anonymous_message(provider, full_request_versions.into_bytes(), 10)
        .await;
    let response = wait_for_control_response(&mut client).await;
    println!("response to 'SupportedRequestVersions' request: {response:#?}");

    client.signal_shutdown()?;
    client.wait_for_shutdown().await;

    Ok(())
}
