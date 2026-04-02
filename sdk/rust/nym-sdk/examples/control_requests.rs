// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Sending control requests to a service provider via the mixnet.
//!
//! Demonstrates `send_message` with explicit SURB counts and the
//! `nym-service-providers-common` request/response protocol. Sends
//! Health, BinaryInfo, and SupportedRequestVersions queries.
//!
//! Run with: cargo run --example control_requests

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
    // Connect an ephemeral client.
    let mut client = MixnetClient::connect_new().await.unwrap();
    let provider: Recipient = "8YF6f8x17j3fviBdU87EGD9g9MAgn9DARxunwLEVM7Bm.4ydfpjbTjCmzj58hWdQjxU2gT6CRVnTbnKajr2hAGBBM@2xU4CBE6QiiYt6EyBXSALwxkNvM7gqJfjHXaMkjiFmYW".parse().unwrap();

    // Build control requests using the service-provider interface.
    let request_health = ControlRequest::Health;
    let request_binary_info = ControlRequest::BinaryInfo;
    let request_versions = ControlRequest::SupportedRequestVersions;

    let full_request_health: Request =
        Request::new_control(ProviderInterfaceVersion::new_current(), request_health);
    let full_request_binary_info: Request =
        Request::new_control(ProviderInterfaceVersion::new_current(), request_binary_info);
    let full_request_versions: Request =
        Request::new_control(ProviderInterfaceVersion::new_current(), request_versions);

    // Send a Health request with 10 reply SURBs and wait for the response.
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

    // Send a BinaryInfo request (no SURBs — replies won't work).
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

    // Send a SupportedRequestVersions request.
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

    // Disconnect for clean shutdown.
    client.disconnect().await;
    Ok(())
}
