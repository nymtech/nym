#[cfg(test)]
mod v0;

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use nym_credential_verification::{ClientBandwidth, TicketVerifier};
    use nym_credentials_interface::CredentialSpendingData;
    use nym_http_api_client::Client;
    use nym_wireguard::{peer_controller::PeerControlRequest, CONTROL_CHANNEL_SIZE};
    use nym_wireguard_private_metadata_client::WireguardMetadataApiClient;
    use nym_wireguard_private_metadata_server::{
        AppState, PeerControllerTransceiver, RouterBuilder,
    };
    use nym_wireguard_private_metadata_shared::{latest, v0, v1, ErrorResponse};
    use tokio::{net::TcpListener, sync::mpsc};

    pub(crate) const VERIFIER_AVAILABLE_BANDWIDTH: i64 = 42;
    pub(crate) const CREDENTIAL_BYTES: [u8; 1245] = [
        0, 0, 4, 133, 96, 179, 223, 185, 136, 23, 213, 166, 59, 203, 66, 69, 209, 181, 227, 254,
        16, 102, 98, 237, 59, 119, 170, 111, 31, 194, 51, 59, 120, 17, 115, 229, 79, 91, 11, 139,
        154, 2, 212, 23, 68, 70, 167, 3, 240, 54, 224, 171, 221, 1, 69, 48, 60, 118, 119, 249, 123,
        35, 172, 227, 131, 96, 232, 209, 187, 123, 4, 197, 102, 90, 96, 45, 125, 135, 140, 99, 1,
        151, 17, 131, 143, 157, 97, 107, 139, 232, 212, 87, 14, 115, 253, 255, 166, 167, 186, 43,
        90, 96, 173, 105, 120, 40, 10, 163, 250, 224, 214, 200, 178, 4, 160, 16, 130, 59, 76, 193,
        39, 240, 3, 101, 141, 209, 183, 226, 186, 207, 56, 210, 187, 7, 164, 240, 164, 205, 37, 81,
        184, 214, 193, 195, 90, 205, 238, 225, 195, 104, 12, 123, 203, 57, 233, 243, 215, 145, 195,
        196, 57, 38, 125, 172, 18, 47, 63, 165, 110, 219, 180, 40, 58, 116, 92, 254, 160, 98, 48,
        92, 254, 232, 107, 184, 80, 234, 60, 160, 235, 249, 76, 41, 38, 165, 28, 40, 136, 74, 48,
        166, 50, 245, 23, 201, 140, 101, 79, 93, 235, 128, 186, 146, 126, 180, 134, 43, 13, 186,
        19, 195, 48, 168, 201, 29, 216, 95, 176, 198, 132, 188, 64, 39, 212, 150, 32, 52, 53, 38,
        228, 199, 122, 226, 217, 75, 40, 191, 151, 48, 164, 242, 177, 79, 14, 122, 105, 151, 85,
        88, 199, 162, 17, 96, 103, 83, 178, 128, 9, 24, 30, 74, 108, 241, 85, 240, 166, 97, 241,
        85, 199, 11, 198, 226, 234, 70, 107, 145, 28, 208, 114, 51, 12, 234, 108, 101, 202, 112,
        48, 185, 22, 159, 67, 109, 49, 27, 149, 90, 109, 32, 226, 112, 7, 201, 208, 209, 104, 31,
        97, 134, 204, 145, 27, 181, 206, 181, 106, 32, 110, 136, 115, 249, 201, 111, 5, 245, 203,
        71, 121, 169, 126, 151, 178, 236, 59, 221, 195, 48, 135, 115, 6, 50, 227, 74, 97, 107, 107,
        213, 90, 2, 203, 154, 138, 47, 128, 52, 134, 128, 224, 51, 65, 240, 90, 8, 55, 175, 180,
        178, 204, 206, 168, 110, 51, 57, 189, 169, 48, 169, 136, 121, 99, 51, 170, 178, 214, 74, 1,
        96, 151, 167, 25, 173, 180, 171, 155, 10, 55, 142, 234, 190, 113, 90, 79, 80, 244, 71, 166,
        30, 235, 113, 150, 133, 1, 218, 17, 109, 111, 223, 24, 216, 177, 41, 2, 204, 65, 221, 212,
        207, 236, 144, 6, 65, 224, 55, 42, 1, 1, 161, 134, 118, 127, 111, 220, 110, 127, 240, 71,
        223, 129, 12, 93, 20, 220, 60, 56, 71, 146, 184, 95, 132, 69, 28, 56, 53, 192, 213, 22,
        119, 230, 152, 225, 182, 188, 163, 219, 37, 175, 247, 73, 14, 247, 38, 72, 243, 1, 48, 131,
        59, 8, 13, 96, 143, 185, 127, 241, 161, 217, 24, 149, 193, 40, 16, 30, 202, 151, 28, 119,
        240, 153, 101, 156, 61, 193, 72, 245, 199, 181, 12, 231, 65, 166, 67, 142, 121, 207, 202,
        58, 197, 113, 188, 248, 42, 124, 105, 48, 161, 241, 55, 209, 36, 194, 27, 63, 233, 144,
        189, 85, 117, 234, 9, 139, 46, 31, 206, 114, 95, 131, 29, 240, 13, 81, 142, 140, 133, 33,
        30, 41, 141, 37, 80, 217, 95, 221, 76, 115, 86, 201, 165, 51, 252, 9, 28, 209, 1, 48, 150,
        74, 248, 212, 187, 222, 66, 210, 3, 200, 19, 217, 171, 184, 42, 148, 53, 150, 57, 50, 6,
        227, 227, 62, 49, 42, 148, 148, 157, 82, 191, 58, 24, 34, 56, 98, 120, 89, 105, 176, 85,
        15, 253, 241, 41, 153, 195, 136, 1, 48, 142, 126, 213, 101, 223, 79, 133, 230, 105, 38,
        161, 149, 2, 21, 136, 150, 42, 72, 218, 85, 146, 63, 223, 58, 108, 186, 183, 248, 62, 20,
        47, 34, 113, 160, 177, 204, 181, 16, 24, 212, 224, 35, 84, 51, 168, 56, 136, 11, 1, 48,
        135, 242, 62, 149, 230, 178, 32, 224, 119, 26, 234, 163, 237, 224, 114, 95, 112, 140, 170,
        150, 96, 125, 136, 221, 180, 78, 18, 11, 12, 184, 2, 198, 217, 119, 43, 69, 4, 172, 109,
        55, 183, 40, 131, 172, 161, 88, 183, 101, 1, 48, 173, 216, 22, 73, 42, 255, 211, 93, 249,
        87, 159, 115, 61, 91, 55, 130, 17, 216, 60, 34, 122, 55, 8, 244, 244, 153, 151, 57, 5, 144,
        178, 55, 249, 64, 211, 168, 34, 148, 56, 89, 92, 203, 70, 124, 219, 152, 253, 165, 0, 32,
        203, 116, 63, 7, 240, 222, 82, 86, 11, 149, 167, 72, 224, 55, 190, 66, 201, 65, 168, 184,
        96, 47, 194, 241, 168, 124, 7, 74, 214, 250, 37, 76, 32, 218, 69, 122, 103, 215, 145, 169,
        24, 212, 229, 168, 106, 10, 144, 31, 13, 25, 178, 242, 250, 106, 159, 40, 48, 163, 165, 61,
        130, 57, 146, 4, 73, 32, 254, 233, 125, 135, 212, 29, 111, 4, 177, 114, 15, 210, 170, 82,
        108, 110, 62, 166, 81, 209, 106, 176, 156, 14, 133, 242, 60, 127, 120, 242, 28, 97, 0, 1,
        32, 103, 93, 109, 89, 240, 91, 1, 84, 150, 50, 206, 157, 203, 49, 220, 120, 234, 175, 234,
        150, 126, 225, 94, 163, 164, 199, 138, 114, 62, 99, 106, 112, 1, 32, 171, 40, 220, 82, 241,
        203, 76, 146, 111, 139, 182, 179, 237, 182, 115, 75, 128, 201, 107, 43, 214, 0, 135, 217,
        160, 68, 150, 232, 144, 114, 237, 98, 32, 30, 134, 232, 59, 93, 163, 253, 244, 13, 202, 52,
        147, 168, 83, 121, 123, 95, 21, 210, 209, 225, 223, 143, 49, 10, 205, 238, 1, 22, 83, 81,
        70, 1, 32, 26, 76, 6, 234, 160, 50, 139, 102, 161, 232, 155, 106, 130, 171, 226, 210, 233,
        178, 85, 247, 71, 123, 55, 53, 46, 67, 148, 137, 156, 207, 208, 107, 1, 32, 102, 31, 4, 98,
        110, 156, 144, 61, 229, 140, 198, 84, 196, 238, 128, 35, 131, 182, 137, 125, 241, 95, 69,
        131, 170, 27, 2, 144, 75, 72, 242, 102, 3, 32, 121, 80, 45, 173, 56, 65, 218, 27, 40, 251,
        197, 32, 169, 104, 123, 110, 90, 78, 153, 166, 38, 9, 129, 228, 99, 8, 1, 116, 142, 233,
        162, 69, 32, 216, 169, 159, 116, 95, 12, 63, 176, 195, 6, 183, 123, 135, 75, 61, 112, 106,
        83, 235, 176, 41, 27, 248, 48, 71, 165, 170, 12, 92, 103, 103, 81, 32, 58, 74, 75, 145,
        192, 94, 153, 69, 80, 128, 241, 3, 16, 117, 192, 86, 161, 103, 44, 174, 211, 196, 182, 124,
        55, 11, 107, 142, 49, 88, 6, 41, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
        6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 0, 37, 139, 240, 0, 0,
        0, 0, 0, 0, 0, 1,
    ];

    pub(crate) struct MockVerifier {
        ret: i64,
    }

    impl MockVerifier {
        pub(crate) fn new(ret: i64) -> MockVerifier {
            Self { ret }
        }
    }

    #[async_trait::async_trait]
    impl TicketVerifier for MockVerifier {
        async fn verify(&mut self) -> nym_credential_verification::Result<i64> {
            Ok(self.ret)
        }
    }

    pub(crate) async fn spawn_server_and_create_client() -> Client {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (request_tx, mut request_rx) = mpsc::channel(CONTROL_CHANNEL_SIZE);
        let router = RouterBuilder::with_default_routes()
            .with_state(AppState::new(PeerControllerTransceiver::new(request_tx)))
            .router;

        tokio::spawn(async move {
            loop {
                match request_rx.recv().await {
                    Some(PeerControlRequest::GetClientBandwidthByIp { ip: _, response_tx }) => {
                        response_tx
                            .send(Ok(ClientBandwidth::new(Default::default())))
                            .ok();
                    }
                    Some(PeerControlRequest::GetVerifierByIp {
                        ip: _,
                        credential: _,
                        response_tx,
                    }) => {
                        response_tx
                            .send(Ok(Box::new(MockVerifier::new(
                                VERIFIER_AVAILABLE_BANDWIDTH,
                            ))))
                            .ok();
                    }
                    None => break,
                    _ => panic!("Not expected"),
                }
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
    async fn query_latest_version() {
        let client = spawn_server_and_create_client().await;
        let version = client.version().await.unwrap();
        assert_eq!(version, latest::VERSION);
    }

    #[tokio::test]
    async fn query_against_server_v0() {
        let client = super::v0::network::test::spawn_server_and_create_client().await;

        // version check
        let version = client.version().await.unwrap();
        assert_eq!(version, v0::VERSION);

        // v0 reqwests
        let request = v0::AvailableBandwidthRequest {}.try_into().unwrap();
        let response = client.available_bandwidth(&request).await.unwrap();
        v0::AvailableBandwidthResponse::try_from(response).unwrap();

        let request = v0::TopUpRequest {}.try_into().unwrap();
        let response = client.topup_bandwidth(&request).await.unwrap();
        v0::TopUpResponse::try_from(response).unwrap();

        // v1 reqwests
        let request = v1::AvailableBandwidthRequest {}.try_into().unwrap();
        assert!(client.available_bandwidth(&request).await.is_err());

        let request = v1::TopUpRequest {
            credential: CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap(),
        }
        .try_into()
        .unwrap();
        assert!(client.topup_bandwidth(&request).await.is_err());
    }

    #[tokio::test]
    async fn query_against_server_v1() {
        let client = spawn_server_and_create_client().await;

        // version check
        let version = client.version().await.unwrap();
        assert_eq!(version, v1::VERSION);

        // v0 reqwests
        let request = v0::AvailableBandwidthRequest {}.try_into().unwrap();
        let response = client.available_bandwidth(&request).await.unwrap();
        v0::AvailableBandwidthResponse::try_from(response).unwrap();

        let request = v0::TopUpRequest {}.try_into().unwrap();
        assert!(client.topup_bandwidth(&request).await.is_err());

        // v1 reqwests
        let request = v1::AvailableBandwidthRequest {}.try_into().unwrap();
        let response = client.available_bandwidth(&request).await.unwrap();
        let available_bandwidth = v1::AvailableBandwidthResponse::try_from(response)
            .unwrap()
            .available_bandwidth;
        assert_eq!(available_bandwidth, 0);

        let request = v1::TopUpRequest {
            credential: CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap(),
        }
        .try_into()
        .unwrap();
        let response = client.topup_bandwidth(&request).await.unwrap();
        let available_bandwidth = v1::TopUpResponse::try_from(response)
            .unwrap()
            .available_bandwidth;
        assert_eq!(available_bandwidth, VERIFIER_AVAILABLE_BANDWIDTH);
    }
}
