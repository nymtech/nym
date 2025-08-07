// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "client")]
pub mod client;
pub mod error;
#[cfg(feature = "server")]
mod http;
mod models;
#[cfg(feature = "server")]
mod network;
mod routes;
#[cfg(feature = "server")]
mod transceiver;

#[cfg(feature = "server")]
pub use http::{
    router::{ApiHttpServer, RouterBuilder, RouterWithState},
    state::AppState,
    ShutdownHandles,
};
pub use models::{v1, ErrorResponse, Version};
#[cfg(feature = "server")]
pub use transceiver::PeerControllerTransceiver;

fn make_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::net::SocketAddr;

    use nym_credential_verification::ClientBandwidth;
    use nym_http_api_client::Client;
    use nym_wireguard::{peer_controller::PeerControlRequest, CONTROL_CHANNEL_SIZE};
    use tokio::{net::TcpListener, sync::mpsc};

    use crate::{
        client::WireguardMetadataApiClient, models::ErrorResponse,
        transceiver::tests::MockVerifier, AppState, PeerControllerTransceiver, RouterBuilder,
    };

    pub(crate) const VERIFIER_AVAILABLE_BANDWIDTH: i64 = 42;

    pub(crate) async fn spawn_server_and_create_client() -> Client {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (request_tx, mut request_rx) = mpsc::channel(CONTROL_CHANNEL_SIZE);
        let router = RouterBuilder::with_default_routes()
            .with_state(AppState::new(PeerControllerTransceiver::new(request_tx)))
            .router;

        tokio::spawn(async move {
            match request_rx.recv().await.unwrap() {
                PeerControlRequest::GetClientBandwidthByIp { ip: _, response_tx } => {
                    response_tx
                        .send(Ok(ClientBandwidth::new(Default::default())))
                        .ok();
                }
                PeerControlRequest::GetVerifierByIp {
                    ip: _,
                    credential: _,
                    response_tx,
                } => {
                    response_tx
                        .send(Ok(Box::new(MockVerifier::new(
                            VERIFIER_AVAILABLE_BANDWIDTH,
                        ))))
                        .ok();
                }
                _ => unimplemented!(),
            }
        });

        tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        Client::new_url::<_, ErrorResponse>(addr.to_string(), None).unwrap()
    }

    #[tokio::test]
    async fn query_version() {
        let client = spawn_server_and_create_client().await;
        let version = client.version().await.unwrap();
        assert_eq!(version, models::latest::VERSION);
    }
}
