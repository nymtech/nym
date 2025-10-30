// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
pub(crate) mod test {
    use std::net::SocketAddr;

    use crate::tests::{MockVerifier, VERIFIER_AVAILABLE_BANDWIDTH};
    use crate::v1::app_state::AppStateV1;
    use axum::extract::{ConnectInfo, State};
    use axum::{Json, Router, extract::Query};
    use nym_credential_verification::ClientBandwidth;
    use nym_http_api_client::Client;
    use nym_http_api_common::{FormattedResponse, OutputParams};
    use nym_wireguard::{CONTROL_CHANNEL_SIZE, peer_controller::PeerControlRequest};
    use nym_wireguard_private_metadata_server::PeerControllerTransceiver;
    use nym_wireguard_private_metadata_shared::v1::interface::{RequestData, ResponseData};
    use nym_wireguard_private_metadata_shared::{
        AxumErrorResponse, AxumResult, Construct, Extract, Request, Response, v1 as latest,
    };
    use tokio::sync::mpsc::Receiver;
    use tokio::{net::TcpListener, sync::mpsc};
    use tower_http::compression::CompressionLayer;

    fn bandwidth_routes() -> Router<AppStateV1> {
        Router::new()
            .route("/version", axum::routing::get(version))
            .route("/available", axum::routing::post(available_bandwidth))
            .route("/topup", axum::routing::post(topup_bandwidth))
            .layer(CompressionLayer::new())
    }

    async fn version(Query(output): Query<OutputParams>) -> AxumResult<FormattedResponse<u64>> {
        let output = output.output.unwrap_or_default();
        Ok(output.to_response(latest::VERSION.into()))
    }

    async fn available_bandwidth(
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
        Query(output): Query<OutputParams>,
        State(state): State<AppStateV1>,
        Json(request): Json<Request>,
    ) -> AxumResult<FormattedResponse<Response>> {
        let output = output.output.unwrap_or_default();

        let (RequestData::AvailableBandwidth(_), version) =
            request.extract().map_err(AxumErrorResponse::bad_request)?
        else {
            return Err(AxumErrorResponse::bad_request("incorrect request type"));
        };
        let available_bandwidth = state
            .available_bandwidth(addr.ip())
            .await
            .map_err(AxumErrorResponse::bad_request)?;
        let response = Response::construct(
            ResponseData::AvailableBandwidth(available_bandwidth),
            version,
        )
        .map_err(AxumErrorResponse::bad_request)?;

        Ok(output.to_response(response))
    }

    async fn topup_bandwidth(
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
        Query(output): Query<OutputParams>,
        State(state): State<AppStateV1>,
        Json(request): Json<Request>,
    ) -> AxumResult<FormattedResponse<Response>> {
        let output = output.output.unwrap_or_default();

        let (RequestData::TopUpBandwidth(credential), version) =
            request.extract().map_err(AxumErrorResponse::bad_request)?
        else {
            return Err(AxumErrorResponse::bad_request("incorrect request type"));
        };
        let available_bandwidth = state
            .topup_bandwidth(addr.ip(), *credential)
            .await
            .map_err(AxumErrorResponse::bad_request)?;
        let response =
            Response::construct(ResponseData::TopUpBandwidth(available_bandwidth), version)
                .map_err(AxumErrorResponse::bad_request)?;

        Ok(output.to_response(response))
    }

    fn spawn_mock_peer_controller(mut request_rx: Receiver<PeerControlRequest>) {
        tokio::spawn(async move {
            while let Some(request) = request_rx.recv().await {
                match request {
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
                    _ => panic!("Not expected"),
                }
            }
        });
    }

    pub(crate) async fn spawn_server_and_create_client() -> Client {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (request_tx, request_rx) = mpsc::channel(CONTROL_CHANNEL_SIZE);
        let router = Router::new()
            .nest("/v1", Router::new().nest("/bandwidth", bandwidth_routes()))
            .with_state(AppStateV1::new(PeerControllerTransceiver::new(request_tx)));

        spawn_mock_peer_controller(request_rx);

        tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        Client::new_url(addr.to_string(), None).unwrap()
    }
}
