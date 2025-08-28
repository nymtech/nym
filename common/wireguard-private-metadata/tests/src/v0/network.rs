// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
pub(crate) mod test {
    use std::net::SocketAddr;

    use crate::{
        tests::{MockVerifier, VERIFIER_AVAILABLE_BANDWIDTH},
        v0::interface::{RequestData, ResponseData},
    };
    use axum::{extract::Query, Json, Router};
    use nym_credential_verification::ClientBandwidth;
    use nym_http_api_client::Client;
    use nym_http_api_common::{FormattedResponse, OutputParams};
    use nym_wireguard::{peer_controller::PeerControlRequest, CONTROL_CHANNEL_SIZE};
    use nym_wireguard_private_metadata_server::PeerControllerTransceiver;
    use nym_wireguard_private_metadata_shared::ErrorResponse;
    use nym_wireguard_private_metadata_shared::{
        v0 as latest, AxumErrorResponse, AxumResult, Construct, Extract, Request, Response,
    };
    use tokio::{net::TcpListener, sync::mpsc};
    use tower_http::compression::CompressionLayer;

    use nym_wireguard_private_metadata_server::AppState;

    fn bandwidth_routes() -> Router<AppState> {
        Router::new()
            .route("/version", axum::routing::get(version))
            .route("/available", axum::routing::post(available_bandwidth))
            .route("/topup", axum::routing::post(topup_bandwidth))
            .layer(CompressionLayer::new())
    }

    #[utoipa::path(
    tag = "bandwidth",
    get,
    path = "/v1/bandwidth/version",
    responses(
        (status = 200, content(
            (Response = "application/bincode")
        ))
    ),
)]
    async fn version(Query(output): Query<OutputParams>) -> AxumResult<FormattedResponse<u64>> {
        let output = output.output.unwrap_or_default();
        Ok(output.to_response(latest::VERSION.into()))
    }

    #[utoipa::path(
    tag = "bandwidth",
    post,
    request_body = Request,
    path = "/v1/bandwidth/available",
    responses(
        (status = 200, content(
            (Response = "application/bincode")
        ))
    ),
)]
    async fn available_bandwidth(
        Query(output): Query<OutputParams>,
        Json(request): Json<Request>,
    ) -> AxumResult<FormattedResponse<Response>> {
        let output = output.output.unwrap_or_default();

        let (RequestData::AvailableBandwidth(_), version) =
            request.extract().map_err(AxumErrorResponse::bad_request)?
        else {
            return Err(AxumErrorResponse::bad_request("incorrect request type"));
        };
        let response = Response::construct(ResponseData::AvailableBandwidth(()), version)
            .map_err(AxumErrorResponse::bad_request)?;

        Ok(output.to_response(response))
    }

    #[utoipa::path(
    tag = "bandwidth",
    post,
    request_body = Request,
    path = "/v1/bandwidth/topup",
    responses(
        (status = 200, content(
            (Response = "application/bincode")
        ))
    ),
)]
    async fn topup_bandwidth(
        Query(output): Query<OutputParams>,
        Json(request): Json<Request>,
    ) -> AxumResult<FormattedResponse<Response>> {
        let output = output.output.unwrap_or_default();

        let (RequestData::TopUpBandwidth(_), version) =
            request.extract().map_err(AxumErrorResponse::bad_request)?
        else {
            return Err(AxumErrorResponse::bad_request("incorrect request type"));
        };
        let response = Response::construct(ResponseData::TopUpBandwidth(()), version)
            .map_err(AxumErrorResponse::bad_request)?;

        Ok(output.to_response(response))
    }

    pub(crate) async fn spawn_server_and_create_client() -> Client {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (request_tx, mut request_rx) = mpsc::channel(CONTROL_CHANNEL_SIZE);
        let router = Router::new()
            .nest("/v1", Router::new().nest("/bandwidth", bandwidth_routes()))
            .with_state(AppState::new(PeerControllerTransceiver::new(request_tx)));

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
                _ => panic!("Not expected"),
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
}
