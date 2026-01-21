// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// use nym_client::client::config::{BaseClientConfig, Config, GatewayEndpointConfig};
// use nym_client::client::{DirectClient, KeyManager, Recipient, ReconstructedMessage, SocketClient};
use nym_sdk::mixnet::{
    IncludedSurbs, MixnetClient, MixnetMessageSender, Recipient, ReconstructedMessage,
};
use nym_service_providers_common::interface::{
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

async fn wait_for_control_response(client: &mut MixnetClient) -> ControlResponse {
    loop {
        let next = client.wait_for_messages().await.unwrap();
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
    let mut client = MixnetClient::connect_new().await.unwrap();
    let provider: Recipient = "8YF6f8x17j3fviBdU87EGD9g9MAgn9DARxunwLEVM7Bm.4ydfpjbTjCmzj58hWdQjxU2gT6CRVnTbnKajr2hAGBBM@2xU4CBE6QiiYt6EyBXSALwxkNvM7gqJfjHXaMkjiFmYW".parse().unwrap();

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
    println!("Sending 'Health' request...");
    client
        .send_message(
            provider,
            full_request_health.into_bytes(),
            IncludedSurbs::new(10),
        )
        .await?;
    let response = wait_for_control_response(&mut client).await;
    println!("response to 'Health' request: {response:#?}");

    println!("Sending 'BinaryInfo' request...");
    client
        .send_message(
            provider,
            full_request_binary_info.into_bytes(),
            IncludedSurbs::none(),
        )
        .await?;
    let response = wait_for_control_response(&mut client).await;
    println!("response to 'BinaryInfo' request: {response:#?}");

    println!("Sending 'SupportedRequestVersions' request...");
    client
        .send_message(
            provider,
            full_request_versions.into_bytes(),
            IncludedSurbs::none(),
        )
        .await?;
    let response = wait_for_control_response(&mut client).await;
    println!("response to 'SupportedRequestVersions' request: {response:#?}");

    client.disconnect().await;
    Ok(())
}
