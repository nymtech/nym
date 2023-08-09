// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sdk::mixnet::{
    IncludedSurbs, MixnetClient, MixnetMessageSender, Recipient, ReconstructedMessage,
};
use nym_service_providers_common::interface::{
    ProviderInterfaceVersion, Request, Response, ResponseContent,
};
use nym_socks5_requests::{QueryRequest, Socks5ProtocolVersion, Socks5Request, Socks5Response};

fn parse_response(received: Vec<ReconstructedMessage>) -> Socks5Response {
    assert_eq!(received.len(), 1);
    let response: Response<Socks5Request> = Response::try_from_bytes(&received[0].message).unwrap();
    match response.content {
        ResponseContent::Control(control) => panic!("unexpected control response: {:?}", control),
        ResponseContent::ProviderData(data) => data,
    }
}

async fn wait_for_response(client: &mut MixnetClient) -> Socks5Response {
    loop {
        let next = client.wait_for_messages().await.unwrap();
        if !next.is_empty() {
            return parse_response(next);
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //nym_bin_common::logging::setup_logging();
    let mut client = MixnetClient::connect_new().await.unwrap();
    let provider: Recipient = "AN8eLxYWFitCkMn92zim3PrPszxJZDYyFFKP7qnnAAew.8UAxL3LwQBis6WpM3GGXaqKGaVdnLCpGJWumHT6KNdTH@77TSuVU8d1oXKbPzjec2xh4i3Wj5WwUyy9Lr36sm8gZm".parse().unwrap();

    let open_proxy_request = Request::new_provider_data(
        ProviderInterfaceVersion::new_current(),
        Socks5Request::new_query(
            Socks5ProtocolVersion::new_current(),
            QueryRequest::OpenProxy,
        ),
    );
    let description_request = Request::new_provider_data(
        ProviderInterfaceVersion::new_current(),
        Socks5Request::new_query(
            Socks5ProtocolVersion::new_current(),
            QueryRequest::Description,
        ),
    );
    println!("Sending 'OpenProxy' query...");
    client
        .send_message(
            provider,
            open_proxy_request.into_bytes(),
            IncludedSurbs::new(10),
        )
        .await?;
    let response = wait_for_response(&mut client).await;
    println!("response to 'OpenProxy' query: {response:#?}");

    println!("Sending 'Description' query...");
    client
        .send_message(
            provider,
            description_request.into_bytes(),
            IncludedSurbs::none(),
        )
        .await?;
    let response = wait_for_response(&mut client).await;
    println!("response to 'Description' query: {response:#?}");

    client.disconnect().await;
    Ok(())
}
