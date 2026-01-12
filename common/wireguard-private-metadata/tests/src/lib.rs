#[cfg(test)]
mod v0;
#[cfg(test)]
mod v1;
#[cfg(test)]
mod v2;

// TODO: we might possibly want to move it to some common crate
// so that it could be re-used by other tests (if needed)
#[cfg(test)]
pub(crate) mod mock_connect_info;

#[cfg(test)]
mod tests {
    use crate::v2::peer_controller::PeerControlRequestTypeV2;
    use nym_credential_verification::upgrade_mode::UpgradeModeEnableError;
    use nym_credential_verification::{ClientBandwidth, TicketVerifier};
    use nym_credentials_interface::{
        AvailableBandwidth, BandwidthCredential, CredentialSpendingData,
    };
    use nym_crypto::asymmetric::ed25519;
    use nym_http_api_client::HttpClientError;
    use nym_upgrade_mode_check::{
        CREDENTIAL_PROXY_JWT_ISSUER, UpgradeModeAttestation,
        generate_jwt_for_upgrade_mode_attestation, generate_new_attestation_with_starting_time,
    };
    use nym_wireguard_private_metadata_client::WireguardMetadataApiClient;
    use nym_wireguard_private_metadata_shared::{v0, v1, v2};
    use std::net::IpAddr;
    use std::time::Duration;
    use time::OffsetDateTime;
    use time::macros::datetime;

    fn unchecked_ip<S: Into<String>>(raw: S) -> IpAddr {
        raw.into().parse().unwrap()
    }

    const HIGH_BANDWIDTH: i64 = 20000000000000;

    const DUMMY_JWT_ISSUER_ED25519_PRIVATE_KEY: [u8; 32] = [
        152, 17, 144, 255, 213, 219, 246, 208, 109, 33, 100, 73, 1, 141, 32, 63, 141, 89, 167, 2,
        52, 215, 241, 219, 200, 18, 159, 241, 76, 111, 42, 32,
    ];

    pub(crate) fn dummy_jwt_issuer_public_key() -> ed25519::PublicKey {
        let private_key =
            ed25519::PrivateKey::from_bytes(&DUMMY_JWT_ISSUER_ED25519_PRIVATE_KEY).unwrap();
        private_key.public_key()
    }

    const DUMMY_ATTESTER_ED25519_PRIVATE_KEY: [u8; 32] = [
        108, 49, 193, 21, 126, 161, 249, 85, 242, 207, 74, 195, 238, 6, 64, 149, 201, 140, 248,
        163, 122, 170, 79, 198, 87, 85, 36, 29, 243, 92, 64, 161,
    ];

    pub(crate) fn dummy_attester_public_key() -> ed25519::PublicKey {
        let private_key =
            ed25519::PrivateKey::from_bytes(&DUMMY_ATTESTER_ED25519_PRIVATE_KEY).unwrap();
        private_key.public_key()
    }

    fn high_bandwidth() -> Result<ClientBandwidth, nym_wireguard::Error> {
        bandwidth_response(HIGH_BANDWIDTH)
    }

    fn low_bandwidth() -> Result<ClientBandwidth, nym_wireguard::Error> {
        bandwidth_response(0)
    }

    fn bandwidth_response(amount: i64) -> Result<ClientBandwidth, nym_wireguard::Error> {
        Ok::<_, nym_wireguard::Error>(ClientBandwidth::new(AvailableBandwidth {
            bytes: amount,
            expiration: OffsetDateTime::from_unix_timestamp(2000000000).unwrap(),
        }))
    }

    fn mock_verifier(
        bandwidth: i64,
    ) -> Result<Box<dyn TicketVerifier + Send + Sync>, nym_wireguard::Error> {
        Ok::<_, nym_wireguard::Error>(
            Box::new(MockVerifier::new(bandwidth)) as Box<dyn TicketVerifier + Send + Sync>
        )
    }

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

    pub(crate) fn mock_upgrade_mode_attestation() -> UpgradeModeAttestation {
        let starting_time = datetime!(2025-10-20 12:00 UTC);

        // just some random, HARDCODED, key
        let key = ed25519::PrivateKey::from_bytes(&DUMMY_ATTESTER_ED25519_PRIVATE_KEY).unwrap();

        generate_new_attestation_with_starting_time(
            &key,
            vec![dummy_jwt_issuer_public_key()],
            starting_time,
        )
    }

    pub(crate) fn mock_different_upgrade_mode_attestation() -> UpgradeModeAttestation {
        let starting_time = datetime!(2025-10-30 12:00 UTC);

        // just some random, HARDCODED, key
        let key = ed25519::PrivateKey::from_bytes(&[
            108, 49, 193, 21, 126, 161, 249, 85, 242, 207, 74, 195, 238, 6, 64, 149, 201, 140, 248,
            163, 122, 170, 79, 198, 87, 85, 36, 29, 243, 92, 64, 161,
        ])
        .unwrap();

        generate_new_attestation_with_starting_time(
            &key,
            vec![dummy_jwt_issuer_public_key()],
            starting_time,
        )
    }

    pub(crate) fn mock_upgrade_mode_jwt() -> String {
        let jwt_key =
            ed25519::PrivateKey::from_bytes(&DUMMY_JWT_ISSUER_ED25519_PRIVATE_KEY).unwrap();
        let keys = ed25519::KeyPair::from(jwt_key);
        // sanity check in case hardcoded values were modified inconsistently
        debug_assert_eq!(*keys.public_key(), dummy_jwt_issuer_public_key());

        let attestation = mock_upgrade_mode_attestation();
        generate_jwt_for_upgrade_mode_attestation(
            attestation,
            Duration::from_secs(60 * 60),
            &keys,
            Some(CREDENTIAL_PROXY_JWT_ISSUER),
        )
    }

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

    // #[tokio::test]
    // async fn query_latest_version() {
    //     let client = super::v2::network::test::spawn_server_and_create_client().await;
    //     let version = client.version().await.unwrap();
    //     assert_eq!(version, latest::VERSION);
    // }

    #[tokio::test]
    async fn query_against_server_v0() {
        let client = super::v0::network::test::spawn_server_and_create_client().await;

        // version check
        let version = client.version().await.unwrap();
        assert_eq!(version, v0::VERSION);

        // v0 requests
        let request = v0::AvailableBandwidthRequest {}.try_into().unwrap();
        let response = client.available_bandwidth(&request).await.unwrap();
        v0::AvailableBandwidthResponse::try_from(response).unwrap();

        let request = v0::TopUpRequest {}.try_into().unwrap();
        let response = client.topup_bandwidth(&request).await.unwrap();
        v0::TopUpResponse::try_from(response).unwrap();

        // v1 requests
        let request = v1::AvailableBandwidthRequest {}.try_into().unwrap();
        assert!(client.available_bandwidth(&request).await.is_err());

        let request = v1::TopUpRequest {
            credential: CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap(),
        }
        .try_into()
        .unwrap();
        assert!(client.topup_bandwidth(&request).await.is_err());

        // v2 requests
        let request = v2::AvailableBandwidthRequest {}.try_into().unwrap();
        assert!(client.available_bandwidth(&request).await.is_err());

        let request = v2::TopUpRequest {
            credential: BandwidthCredential::from(
                CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap(),
            ),
        }
        .try_into()
        .unwrap();
        assert!(client.topup_bandwidth(&request).await.is_err());
    }

    #[tokio::test]
    async fn query_against_server_v1() {
        let client = super::v1::network::test::spawn_server_and_create_client().await;

        // version check
        let version = client.version().await.unwrap();
        assert_eq!(version, v1::VERSION);

        // v0 requests
        let request = v0::AvailableBandwidthRequest {}.try_into().unwrap();
        let response = client.available_bandwidth(&request).await.unwrap();
        v0::AvailableBandwidthResponse::try_from(response).unwrap();

        let request = v0::TopUpRequest {}.try_into().unwrap();
        assert!(client.topup_bandwidth(&request).await.is_err());

        // v1 requests
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

        // v2 requests
        let request = v2::AvailableBandwidthRequest {}.try_into().unwrap();
        assert!(client.available_bandwidth(&request).await.is_err());

        let request = v2::TopUpRequest {
            credential: BandwidthCredential::from(
                CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap(),
            ),
        }
        .try_into()
        .unwrap();
        assert!(client.topup_bandwidth(&request).await.is_err());
    }

    #[tokio::test]
    async fn query_against_server_v2() {
        let server_test = super::v2::network::test::spawn_server_and_create_client().await;
        let client = &server_test.api_client;

        // version check
        let version = client.version().await.unwrap();
        assert_eq!(version, v2::VERSION);

        // ===========
        // v0 requests
        // ===========
        let client_ip = unchecked_ip("0.0.0.1");
        server_test.set_client_ip(client_ip);
        server_test
            .register_peer_controller_response(
                PeerControlRequestTypeV2::GetClientBandwidthByIp { ip: client_ip },
                bandwidth_response(0),
            )
            .await;

        server_test
            .register_peer_controller_response(
                PeerControlRequestTypeV2::GetVerifierByIp { ip: client_ip },
                mock_verifier(10),
            )
            .await;

        let request = v0::AvailableBandwidthRequest {}.try_into().unwrap();
        let response = client.available_bandwidth(&request).await.unwrap();
        v0::AvailableBandwidthResponse::try_from(response).unwrap();

        let request = v0::TopUpRequest {}.try_into().unwrap();
        assert!(client.topup_bandwidth(&request).await.is_err());
        server_test.reset_registered_responses().await;

        // ===========
        // v1 requests
        // ===========
        let client_ip = unchecked_ip("1.1.1.1");
        server_test.set_client_ip(client_ip);
        server_test
            .register_peer_controller_response(
                PeerControlRequestTypeV2::GetClientBandwidthByIp { ip: client_ip },
                bandwidth_response(0),
            )
            .await;

        server_test
            .register_peer_controller_response(
                PeerControlRequestTypeV2::GetVerifierByIp { ip: client_ip },
                mock_verifier(100),
            )
            .await;

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
        assert_eq!(available_bandwidth, 100);
        server_test.reset_registered_responses().await;

        // ===========
        // v2 requests
        // ===========
        let client_ip = unchecked_ip("2.2.2.1");
        server_test.set_client_ip(client_ip);
        server_test
            .register_peer_controller_response(
                PeerControlRequestTypeV2::GetClientBandwidthByIp { ip: client_ip },
                bandwidth_response(0),
            )
            .await;

        server_test
            .register_peer_controller_response(
                PeerControlRequestTypeV2::GetVerifierByIp { ip: client_ip },
                mock_verifier(200),
            )
            .await;

        let request = v2::AvailableBandwidthRequest {}.try_into().unwrap();
        let response = client.available_bandwidth(&request).await.unwrap();
        let available = v2::AvailableBandwidthResponse::try_from(response).unwrap();
        assert_eq!(available.available_bandwidth, 0);
        assert!(!available.upgrade_mode);

        let request = v2::TopUpRequest {
            credential: BandwidthCredential::from(
                CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap(),
            ),
        }
        .try_into()
        .unwrap();
        let response = client.topup_bandwidth(&request).await.unwrap();
        let top_up = v2::TopUpResponse::try_from(response).unwrap();
        assert_eq!(top_up.available_bandwidth, 200);
        assert!(!top_up.upgrade_mode);
        server_test.reset_registered_responses().await;

        // upgrade mode test
        let upgrade_mode_client = unchecked_ip("2.2.2.2");
        server_test.set_client_ip(upgrade_mode_client);
        let good_attestation_alt = mock_different_upgrade_mode_attestation();
        let good_jwt = mock_upgrade_mode_jwt();

        // 1. send attestation when upgrade mode is not enabled
        let request = v2::TopUpRequest {
            credential: BandwidthCredential::UpgradeModeJWT {
                token: good_jwt.clone(),
            },
        }
        .try_into()
        .unwrap();
        let response_err = client.topup_bandwidth(&request).await.unwrap_err();
        let HttpClientError::EndpointFailure { error, .. } = response_err else {
            panic!("unexpected response")
        };
        assert!(error.contains(&UpgradeModeEnableError::AttestationNotPublished.to_string()));
        server_test.reset_registered_responses().await;

        // 2.1. send attestation when upgrade mode is enabled (low bandwidth)
        let request_typ = PeerControlRequestTypeV2::GetClientBandwidthByIp {
            ip: upgrade_mode_client,
        };
        server_test
            .register_peer_controller_response(request_typ, low_bandwidth())
            .await;
        server_test.enable_upgrade_mode().await;
        let request = v2::TopUpRequest {
            credential: BandwidthCredential::UpgradeModeJWT {
                token: good_jwt.clone(),
            },
        }
        .try_into()
        .unwrap();
        let response = client.topup_bandwidth(&request).await.unwrap();
        let top_up = v2::TopUpResponse::try_from(response).unwrap();
        // as defined by `DEFAULT_WG_CLIENT_BANDWIDTH_THRESHOLD`
        assert_eq!(top_up.available_bandwidth, 1024 * 1024 * 1024);
        assert!(top_up.upgrade_mode);
        server_test.reset_registered_responses().await;

        // 2.2. send attestation when upgrade mode is enabled (high bandwidth)
        let request_typ = PeerControlRequestTypeV2::GetClientBandwidthByIp {
            ip: upgrade_mode_client,
        };
        server_test
            .register_peer_controller_response(request_typ, high_bandwidth())
            .await;
        server_test.enable_upgrade_mode().await;
        let request = v2::TopUpRequest {
            credential: BandwidthCredential::UpgradeModeJWT {
                token: good_jwt.clone(),
            },
        }
        .try_into()
        .unwrap();
        let response = client.topup_bandwidth(&request).await.unwrap();
        let top_up = v2::TopUpResponse::try_from(response).unwrap();
        assert_eq!(top_up.available_bandwidth, HIGH_BANDWIDTH);
        assert!(top_up.upgrade_mode);
        server_test.reset_registered_responses().await;

        // 3. send bad attestation when upgrade mode is enabled
        // (we don't validate it, so client is let through)
        // (the only case where invalid attestation would have been rejected is when server
        // is not aware of the UM, and that was meant to trigger a refresh. however, a test for that
        // is out of scope for these unit tests)
        server_test
            .change_upgrade_mode_attestation(good_attestation_alt)
            .await;
        server_test
            .register_peer_controller_response(request_typ, high_bandwidth())
            .await;
        let request = v2::TopUpRequest {
            credential: BandwidthCredential::UpgradeModeJWT {
                token: good_jwt.clone(),
            },
        }
        .try_into()
        .unwrap();
        let response = client.topup_bandwidth(&request).await.unwrap();
        let top_up = v2::TopUpResponse::try_from(response).unwrap();
        assert_eq!(top_up.available_bandwidth, HIGH_BANDWIDTH);
        assert!(top_up.upgrade_mode);
        server_test.reset_registered_responses().await;

        // 4. send zk-nym when upgrade mode is enabled
        server_test
            .register_peer_controller_response(request_typ, high_bandwidth())
            .await;
        server_test
            .register_peer_controller_response(
                PeerControlRequestTypeV2::GetVerifierByIp {
                    ip: upgrade_mode_client,
                },
                mock_verifier(300),
            )
            .await;
        let request = v2::TopUpRequest {
            credential: BandwidthCredential::from(
                CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap(),
            ),
        }
        .try_into()
        .unwrap();
        let response = client.topup_bandwidth(&request).await.unwrap();
        let top_up = v2::TopUpResponse::try_from(response).unwrap();
        // as defined by `DEFAULT_WG_CLIENT_BANDWIDTH_THRESHOLD`
        assert_eq!(top_up.available_bandwidth, 1024 * 1024 * 1024);
        assert!(top_up.upgrade_mode);
        server_test.reset_registered_responses().await;

        // attempt to enable UM with a valid token
        // no global attestation
        server_test.disable_upgrade_mode().await;
        let request = v2::UpgradeModeCheckRequest {
            request_type: v2::UpgradeModeCheckRequestType::UpgradeModeJwt {
                token: "".to_string(),
            },
        }
        .try_into()
        .unwrap();
        let response = client.request_upgrade_mode_check(&request).await;
        assert!(response.is_err());

        server_test.publish_upgrade_mode_attestation().await;
        // global attestation
        let request = v2::UpgradeModeCheckRequest {
            request_type: v2::UpgradeModeCheckRequestType::UpgradeModeJwt {
                token: mock_upgrade_mode_jwt(),
            },
        }
        .try_into()
        .unwrap();
        let response = client.request_upgrade_mode_check(&request).await.unwrap();
        let upgrade_mode = v2::UpgradeModeCheckResponse::try_from(response).unwrap();
        assert!(upgrade_mode.upgrade_mode);
    }
}
