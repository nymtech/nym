// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
pub(crate) mod test {
    use crate::mock_connect_info::{DummyConnectInfo, MockConnectInfoLayer};
    use crate::tests::mock_upgrade_mode_attestation;
    use crate::v2::app_state::AppStateV2;
    use crate::v2::peer_controller::{
        MockPeerControllerStateV2, MockPeerControllerV2, PeerControlRequestTypeV2,
    };
    use axum::extract::State;
    use axum::{Extension, Json, Router, extract::Query};
    use futures::StreamExt;
    use nym_credential_verification::upgrade_mode::{
        UpgradeModeCheckConfig, UpgradeModeCheckRequestReceiver, UpgradeModeCheckRequestSender,
        UpgradeModeDetails, UpgradeModeState,
    };
    use nym_http_api_client::Client;
    use nym_http_api_common::{FormattedResponse, OutputParams};
    use nym_upgrade_mode_check::UpgradeModeAttestation;
    use nym_wireguard::CONTROL_CHANNEL_SIZE;
    use nym_wireguard_private_metadata_server::AppState;
    use nym_wireguard_private_metadata_server::PeerControllerTransceiver;
    use nym_wireguard_private_metadata_shared::interface::RequestData;
    use nym_wireguard_private_metadata_shared::{
        AxumErrorResponse, AxumResult, Construct, Extract, Request, Response, v2 as latest,
    };
    use std::any::Any;
    use std::net::IpAddr;
    use std::time::Duration;
    use tokio::task::JoinSet;
    use tokio::{net::TcpListener, sync::mpsc};
    use tower_http::compression::CompressionLayer;

    pub struct MockUpgradeModeWatcher {
        check_request_receiver: UpgradeModeCheckRequestReceiver,
    }

    impl MockUpgradeModeWatcher {
        pub fn new(check_request_receiver: UpgradeModeCheckRequestReceiver) -> Self {
            MockUpgradeModeWatcher {
                check_request_receiver,
            }
        }

        pub async fn run(&mut self) {
            // for now don't do anything apart from notifying the caller
            while let Some(request) = self.check_request_receiver.next().await {
                request.finalize()
            }
        }
    }

    pub struct ServerTest {
        // among other things gives you access to the shared state, so you could toggle the flag
        // and thus change server behaviour
        upgrade_mode_state: UpgradeModeState,

        connect_info: DummyConnectInfo,

        // handles to the following tasks:
        // - the actual axum server
        // - dummy attestation watcher
        // - dummy peer controller
        _server_tasks: JoinSet<()>,

        peer_controller_state: MockPeerControllerStateV2,

        pub(crate) api_client: Client,
    }

    impl ServerTest {
        pub(crate) async fn new() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let (request_tx, request_rx) = mpsc::channel(CONTROL_CHANNEL_SIZE);

            let (um_recheck_tx, um_recheck_rx) = futures::channel::mpsc::unbounded();
            let upgrade_mode_state = UpgradeModeState::new_empty();
            let upgrade_mode_details = UpgradeModeDetails::new(
                UpgradeModeCheckConfig {
                    min_staleness_recheck: Duration::from_secs(30),
                },
                UpgradeModeCheckRequestSender::new(um_recheck_tx),
                upgrade_mode_state.clone(),
            );

            let dummy_connect_info = DummyConnectInfo::new();

            let router = Router::new()
                .nest("/v1", Router::new().nest("/bandwidth", bandwidth_routes()))
                .with_state(AppStateV2::new(
                    PeerControllerTransceiver::new(request_tx),
                    upgrade_mode_details,
                ));

            // register responses for expected requests
            let peer_controller_state = MockPeerControllerStateV2::default();
            let mut server_tasks = JoinSet::new();

            let mut peer_controller =
                MockPeerControllerV2::new(peer_controller_state.clone(), request_rx);

            let mut upgrade_mode_watcher = MockUpgradeModeWatcher::new(um_recheck_rx);

            // spawn all the tasks
            server_tasks.spawn(async move {
                peer_controller.run().await;
            });
            server_tasks.spawn(async move {
                upgrade_mode_watcher.run().await;
            });

            let connect_info = dummy_connect_info.clone();
            server_tasks.spawn(async move {
                axum::serve(
                    listener,
                    // router.into_make_service_with_connect_info::<SocketAddr>(),
                    router.layer(MockConnectInfoLayer::new(connect_info)),
                )
                .await
                .unwrap();
            });
            let api_client = Client::new_url(addr.to_string(), None).unwrap();

            ServerTest {
                upgrade_mode_state,
                connect_info: dummy_connect_info,
                _server_tasks: server_tasks,
                peer_controller_state,
                api_client,
            }
        }

        pub(crate) async fn enable_upgrade_mode(&self) {
            self.change_upgrade_mode_attestation(mock_upgrade_mode_attestation())
                .await
        }

        pub(crate) async fn change_upgrade_mode_attestation(
            &self,
            attestation: UpgradeModeAttestation,
        ) {
            self.upgrade_mode_state
                .set_expected_attestation(Some(attestation))
                .await
        }

        #[allow(dead_code)]
        pub(crate) async fn disable_upgrade_mode(&self) {
            self.upgrade_mode_state.set_expected_attestation(None).await;
        }

        pub(crate) fn set_client_ip(&self, ip: IpAddr) {
            self.connect_info.set(ip)
        }

        #[allow(dead_code)]
        pub(crate) fn client_ip(&self) -> IpAddr {
            self.connect_info.ip()
        }

        // note: it's caller's responsibility to make sure the response type is correct!
        pub(crate) async fn register_peer_controller_response(
            &self,
            request: PeerControlRequestTypeV2,
            response: impl Any + Send + Sync + 'static,
        ) {
            self.peer_controller_state
                .register_response(request, response)
                .await
        }

        pub(crate) async fn reset_registered_responses(&self) {
            self.peer_controller_state
                .clear_registered_responses()
                .await
        }
    }

    fn bandwidth_routes() -> Router<AppState> {
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
        // ❗ \/ DIFFERENT FROM ACTUAL SERVER \/ ❗
        // we use different ConnectInfo to be able to mock different ip addresses
        Extension(addr): Extension<DummyConnectInfo>,
        // ❗ /\ DIFFERENT FROM ACTUAL SERVER /\ ❗
        Query(output): Query<OutputParams>,
        State(state): State<AppState>,
        Json(request): Json<Request>,
    ) -> AxumResult<FormattedResponse<Response>> {
        let output = output.output.unwrap_or_default();

        let (RequestData::AvailableBandwidth, version) =
            request.extract().map_err(AxumErrorResponse::bad_request)?
        else {
            return Err(AxumErrorResponse::bad_request("incorrect request type"));
        };
        let available_bandwidth_response = state
            .available_bandwidth(addr.ip())
            .await
            .map_err(AxumErrorResponse::bad_request)?;
        let response = Response::construct(available_bandwidth_response, version)
            .map_err(AxumErrorResponse::bad_request)?;

        Ok(output.to_response(response))
    }

    async fn topup_bandwidth(
        // ❗ \/ DIFFERENT FROM ACTUAL SERVER \/ ❗
        // we use different ConnectInfo to be able to mock different ip addresses
        Extension(addr): Extension<DummyConnectInfo>,
        // ❗ /\ DIFFERENT FROM ACTUAL SERVER /\ ❗
        Query(output): Query<OutputParams>,
        State(state): State<AppState>,
        Json(request): Json<Request>,
    ) -> AxumResult<FormattedResponse<Response>> {
        let output = output.output.unwrap_or_default();

        let (RequestData::TopUpBandwidth { credential }, version) =
            request.extract().map_err(AxumErrorResponse::bad_request)?
        else {
            return Err(AxumErrorResponse::bad_request("incorrect request type"));
        };
        let top_up_bandwidth_response = state
            .topup_bandwidth(addr.ip(), credential)
            .await
            .map_err(AxumErrorResponse::bad_request)?;
        let response = Response::construct(top_up_bandwidth_response, version)
            .map_err(AxumErrorResponse::bad_request)?;

        Ok(output.to_response(response))
    }

    pub(crate) async fn spawn_server_and_create_client() -> ServerTest {
        ServerTest::new().await
    }
}
