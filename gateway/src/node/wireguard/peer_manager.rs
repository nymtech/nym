// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::wireguard::GatewayWireguardError;
use defguard_wireguard_rs::{host::Peer, key::Key};
use futures::channel::oneshot;
use nym_credential_verification::{ClientBandwidth, TicketVerifier};
use nym_credentials_interface::CredentialSpendingData;
use nym_metrics::add_histogram_obs;
use nym_wireguard::peer_controller::IpPair;
use nym_wireguard::{peer_controller::PeerControlRequest, WireguardGatewayData};
use nym_wireguard_types::PeerPublicKey;
use std::time::Instant;
use tracing::error;

// Histogram buckets for WireGuard peer controller channel latency
// Measures time to send request and receive response from peer controller
// Expected: 1ms-100ms for normal operations, up to 2s for slow conditions
const WG_CONTROLLER_LATENCY_BUCKETS: &[f64] = &[
    0.001, // 1ms
    0.005, // 5ms
    0.01,  // 10ms
    0.05,  // 50ms
    0.1,   // 100ms
    0.25,  // 250ms
    0.5,   // 500ms
    1.0,   // 1s
    2.0,   // 2s
];

#[derive(Clone)]
pub struct PeerManager {
    pub(crate) wireguard_gateway_data: WireguardGatewayData,
}

impl PeerManager {
    pub fn new(wireguard_gateway_data: WireguardGatewayData) -> Self {
        PeerManager {
            wireguard_gateway_data,
        }
    }

    pub async fn preallocate_peer_ip_pair(&self) -> Result<IpPair, GatewayWireguardError> {
        let controller_start = Instant::now();
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::PreAllocateIpPair { response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| GatewayWireguardError::PeerInteractionStopped)?;

        let res = response_rx
            .await
            .map_err(|e| {
                GatewayWireguardError::InternalError(format!(
                    "Failed to receive IP allocation: {e}"
                ))
            })?
            .map_err(|e| {
                error!("Failed to allocate IPs from pool: {e}");
                GatewayWireguardError::InternalError(format!("Failed to allocate IPs: {e}"))
            });

        let latency = controller_start.elapsed().as_secs_f64();
        add_histogram_obs!(
            "wg_peer_controller_channel_latency_seconds",
            latency,
            WG_CONTROLLER_LATENCY_BUCKETS
        );

        res
    }

    pub async fn release_ip_pair(&self, ip_pair: IpPair) -> Result<(), GatewayWireguardError> {
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::ReleaseIpPair {
            response_tx,
            ip_pair,
        };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| GatewayWireguardError::PeerInteractionStopped)?;

        response_rx
            .await
            .map_err(|_| GatewayWireguardError::internal("no response for release ip allocation"))?
            .map_err(|err| {
                GatewayWireguardError::InternalError(format!(
                    "releasing ip pair not be performed: {err:?}"
                ))
            })?;

        Ok(())
    }

    pub async fn add_peer(&self, peer: Peer) -> Result<(), GatewayWireguardError> {
        let controller_start = Instant::now();
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::AddPeer { peer, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| GatewayWireguardError::PeerInteractionStopped)?;

        let res = response_rx
            .await
            .map_err(|_| GatewayWireguardError::internal("no response for add peer".to_string()))?
            .map_err(|err| {
                GatewayWireguardError::internal(format!(
                    "adding peer could not be performed: {err:?}"
                ))
            });

        let latency = controller_start.elapsed().as_secs_f64();
        add_histogram_obs!(
            "wg_peer_controller_channel_latency_seconds",
            latency,
            WG_CONTROLLER_LATENCY_BUCKETS
        );

        res
    }

    pub async fn remove_peer(&self, pub_key: PeerPublicKey) -> Result<(), GatewayWireguardError> {
        let controller_start = Instant::now();
        let key = Key::new(pub_key.to_bytes());
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::RemovePeer { key, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| GatewayWireguardError::PeerInteractionStopped)?;

        let res = response_rx
            .await
            .map_err(|_| GatewayWireguardError::internal("no response for remove peer"))?
            .map_err(|err| {
                GatewayWireguardError::InternalError(format!(
                    "removing peer could not be performed: {err:?}"
                ))
            });

        let latency = controller_start.elapsed().as_secs_f64();
        add_histogram_obs!(
            "wg_peer_controller_channel_latency_seconds",
            latency,
            WG_CONTROLLER_LATENCY_BUCKETS
        );

        res
    }

    pub async fn check_active_peer(
        &self,
        pub_key: PeerPublicKey,
    ) -> Result<bool, GatewayWireguardError> {
        let controller_start = Instant::now();
        let key = Key::new(pub_key.to_bytes());
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::CheckActivePeer { key, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| GatewayWireguardError::PeerInteractionStopped)?;

        let res = response_rx
            .await
            .map_err(|_| GatewayWireguardError::internal("no response for check active peer"))?
            .map_err(|err| {
                GatewayWireguardError::InternalError(format!(
                    "check active peer could not be performed: {err:?}"
                ))
            });

        let latency = controller_start.elapsed().as_secs_f64();
        add_histogram_obs!(
            "wg_peer_controller_channel_latency_seconds",
            latency,
            WG_CONTROLLER_LATENCY_BUCKETS
        );

        res
    }

    pub async fn query_peer(
        &self,
        public_key: PeerPublicKey,
    ) -> Result<Option<Peer>, GatewayWireguardError> {
        let controller_start = Instant::now();
        let key = Key::new(public_key.to_bytes());
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::QueryPeer { key, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| GatewayWireguardError::PeerInteractionStopped)?;

        let res = response_rx
            .await
            .map_err(|_| GatewayWireguardError::internal("no response for query peer".to_string()))?
            .map_err(|err| {
                GatewayWireguardError::internal(format!(
                    "querying peer could not be performed: {err:?}"
                ))
            });

        let latency = controller_start.elapsed().as_secs_f64();
        add_histogram_obs!(
            "wg_peer_controller_channel_latency_seconds",
            latency,
            WG_CONTROLLER_LATENCY_BUCKETS
        );

        res
    }

    pub async fn query_bandwidth(
        &self,
        public_key: PeerPublicKey,
    ) -> Result<i64, GatewayWireguardError> {
        let client_bandwidth = self.query_client_bandwidth(public_key).await?;
        Ok(client_bandwidth.available().await)
    }

    pub async fn query_client_bandwidth(
        &self,
        key: PeerPublicKey,
    ) -> Result<ClientBandwidth, GatewayWireguardError> {
        let controller_start = Instant::now();
        let key = Key::new(key.to_bytes());
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::GetClientBandwidthByKey { key, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| GatewayWireguardError::PeerInteractionStopped)?;

        let res = response_rx
            .await
            .map_err(|_| GatewayWireguardError::internal("no response for query client bandwidth"))?
            .map_err(|err| {
                GatewayWireguardError::internal(format!(
                    "querying client bandwidth could not be performed: {err:?}"
                ))
            });

        let latency = controller_start.elapsed().as_secs_f64();
        add_histogram_obs!(
            "wg_peer_controller_channel_latency_seconds",
            latency,
            WG_CONTROLLER_LATENCY_BUCKETS
        );

        res
    }

    pub async fn query_verifier_by_key(
        &self,
        key: PeerPublicKey,
        credential: CredentialSpendingData,
    ) -> Result<Box<dyn TicketVerifier + Send + Sync>, GatewayWireguardError> {
        let controller_start = Instant::now();
        let key = Key::new(key.to_bytes());
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::GetVerifierByKey {
            key,
            credential: Box::new(credential),
            response_tx,
        };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| GatewayWireguardError::PeerInteractionStopped)?;

        let res = response_rx
            .await
            .map_err(|_| {
                GatewayWireguardError::internal("no response for query verifier".to_string())
            })?
            .map_err(|err| {
                GatewayWireguardError::internal(format!(
                    "querying verifier could not be performed: {err:?}"
                ))
            });

        let latency = controller_start.elapsed().as_secs_f64();
        add_histogram_obs!(
            "wg_peer_controller_channel_latency_seconds",
            latency,
            WG_CONTROLLER_LATENCY_BUCKETS
        );

        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::wireguard::PeerRegistrator;
    use crate::nym_authenticator::config::Authenticator;
    use defguard_wireguard_rs::net::IpAddrMask;
    use nym_credential_verification::upgrade_mode::testing::mock_dummy_upgrade_mode_details;
    use nym_credential_verification::{
        bandwidth_storage_manager::BandwidthStorageManager, ecash::MockEcashManager,
    };
    use nym_credentials_interface::Bandwidth;
    use nym_crypto::asymmetric::x25519::KeyPair;
    use nym_gateway_storage::traits::{mock::MockGatewayStorage, BandwidthGatewayStorage};
    use nym_task::ShutdownManager;
    use nym_test_utils::helpers::{deterministic_rng, DeterministicRng, RngCore};
    use nym_wireguard::peer_controller::{start_controller, stop_controller};
    use std::{str::FromStr, sync::Arc};
    use time::{Duration, OffsetDateTime};
    use tokio::sync::RwLock;

    const CREDENTIAL_BYTES: [u8; 1245] = [
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

    struct TestSetup {
        rng: DeterministicRng,
        _ecash_manager: Arc<MockEcashManager>,
        storage: Arc<RwLock<MockGatewayStorage>>,
        peer_registrator: PeerRegistrator,
        peer_manager: PeerManager,
        task_manager: ShutdownManager,
    }

    struct GeneratedPeer {
        peer: Peer,
        client_id: i64,
    }

    impl GeneratedPeer {
        fn key(&self) -> PeerPublicKey {
            PeerPublicKey::from_str(self.peer.public_key.to_string().as_str()).unwrap()
        }
    }

    impl TestSetup {
        fn new() -> TestSetup {
            let mut rng = deterministic_rng();
            let (wireguard_data, request_rx) = WireguardGatewayData::new(
                Authenticator::default().into(),
                Arc::new(KeyPair::new(&mut rng)),
            );

            let (upgrade_mode_details, _) = mock_dummy_upgrade_mode_details();
            let peer_manager = PeerManager::new(wireguard_data);

            let (storage, task_manager) = start_controller(
                peer_manager.wireguard_gateway_data.peer_tx().clone(),
                request_rx,
            );

            let ecash_manager = Arc::new(MockEcashManager::new(Box::new(storage.clone())));
            let peer_registrator = PeerRegistrator::new(
                ecash_manager.clone(),
                peer_manager.clone(),
                upgrade_mode_details,
            );

            TestSetup {
                rng,
                _ecash_manager: ecash_manager,
                storage,
                peer_registrator,
                peer_manager,
                task_manager,
            }
        }

        async fn peer_with_pre_allocated_ip(&mut self) -> Peer {
            let mut peer = Peer::default();
            let mut key = [0u8; 32];
            self.rng.fill_bytes(&mut key);
            peer.public_key = Key::new(key);

            let allocation = self.peer_manager.preallocate_peer_ip_pair().await.unwrap();
            peer.allowed_ips = vec![
                IpAddrMask::new(allocation.ipv4.into(), 32),
                IpAddrMask::new(allocation.ipv6.into(), 128),
            ];

            peer
        }

        async fn _add_peer(&self, peer: &Peer) -> i64 {
            let client_id = self
                .storage
                .insert_wireguard_peer(peer, FromStr::from_str("entry_wireguard").unwrap())
                .await
                .unwrap();
            self.peer_registrator
                .credential_storage_preparation(client_id)
                .await
                .unwrap();
            self.peer_manager.add_peer(peer.clone()).await.unwrap();
            client_id
        }

        async fn add_peer(&mut self) -> GeneratedPeer {
            let peer = self.peer_with_pre_allocated_ip().await;
            let client_id = self._add_peer(&peer).await;

            GeneratedPeer { peer, client_id }
        }

        async fn finish(self) {
            stop_controller(self.task_manager).await
        }
    }

    #[tokio::test]
    async fn assign_peer_ip() -> anyhow::Result<()> {
        let test = TestSetup::new();

        let ip_pair1 = test.peer_manager.preallocate_peer_ip_pair().await?;
        let ip_pair2 = test.peer_manager.preallocate_peer_ip_pair().await?;
        assert_ne!(ip_pair1, ip_pair2);

        test.finish().await;

        Ok(())
    }

    #[tokio::test]
    async fn add_peer() {
        let mut test = TestSetup::new();
        let peer = test.peer_with_pre_allocated_ip().await;

        assert!(test.peer_manager.add_peer(peer.clone()).await.is_err());

        let client_id = test
            .storage
            .insert_wireguard_peer(&peer, FromStr::from_str("entry_wireguard").unwrap())
            .await
            .unwrap();
        assert!(test.peer_manager.add_peer(peer.clone()).await.is_err());

        test.peer_registrator
            .credential_storage_preparation(client_id)
            .await
            .unwrap();
        test.peer_manager.add_peer(peer.clone()).await.unwrap();

        test.finish().await
    }

    #[tokio::test]
    async fn remove_peer() {
        let mut test = TestSetup::new();
        let peer = test.add_peer().await;
        let public_key = peer.key();

        test.peer_manager.remove_peer(public_key).await.unwrap();

        test.finish().await
    }

    #[tokio::test]
    async fn query_peer() {
        let mut test = TestSetup::new();
        let peer = test.peer_with_pre_allocated_ip().await;
        let public_key = PeerPublicKey::from_str(peer.public_key.to_string().as_str()).unwrap();

        assert!(test
            .peer_manager
            .query_peer(public_key)
            .await
            .unwrap()
            .is_none());

        test._add_peer(&peer).await;
        let peer_query = test
            .peer_manager
            .query_peer(public_key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(peer.public_key, peer_query.public_key);

        test.finish().await
    }

    #[tokio::test]
    async fn query_bandwidth() {
        let mut test = TestSetup::new();
        let peer = test.peer_with_pre_allocated_ip().await;
        let public_key = PeerPublicKey::from_str(peer.public_key.to_string().as_str()).unwrap();

        assert!(test.peer_manager.query_bandwidth(public_key).await.is_err());

        test._add_peer(&peer).await;
        let available_bandwidth = test.peer_manager.query_bandwidth(public_key).await.unwrap();
        assert_eq!(available_bandwidth, 0);

        test.finish().await
    }

    #[tokio::test]
    async fn query_client_bandwidth() {
        let mut test = TestSetup::new();
        let peer = test.peer_with_pre_allocated_ip().await;
        let public_key = PeerPublicKey::from_str(peer.public_key.to_string().as_str()).unwrap();

        assert!(test
            .peer_manager
            .query_client_bandwidth(public_key)
            .await
            .is_err());

        test._add_peer(&peer).await;
        let available_bandwidth = test
            .peer_manager
            .query_client_bandwidth(public_key)
            .await
            .unwrap()
            .available()
            .await;
        assert_eq!(available_bandwidth, 0);

        test.finish().await
    }

    #[tokio::test]
    async fn query_verifier() {
        let mut test = TestSetup::new();
        let peer = test.peer_with_pre_allocated_ip().await;
        let public_key = PeerPublicKey::from_str(peer.public_key.to_string().as_str()).unwrap();

        let credential = CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap();

        assert!(test
            .peer_manager
            .query_verifier_by_key(public_key, credential.clone())
            .await
            .is_err());

        test._add_peer(&peer).await;
        test.peer_manager
            .query_verifier_by_key(public_key, credential)
            .await
            .unwrap();

        test.finish().await
    }

    #[tokio::test]
    async fn increase_decrease_bandwidth() {
        let mut test = TestSetup::new();
        let peer = test.add_peer().await;
        let public_key = peer.key();

        let top_up = 42;
        let consume = 4;

        let client_bandwidth = test
            .peer_manager
            .query_client_bandwidth(peer.key())
            .await
            .unwrap();

        let mut bw_manager = BandwidthStorageManager::new(
            Box::new(test.storage.clone()),
            client_bandwidth.clone(),
            peer.client_id,
            Default::default(),
            true,
        );
        bw_manager
            .increase_bandwidth(
                Bandwidth::new_unchecked(top_up as u64),
                OffsetDateTime::now_utc()
                    .checked_add(Duration::minutes(1))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(client_bandwidth.available().await, top_up);
        assert_eq!(
            test.peer_manager.query_bandwidth(public_key).await.unwrap(),
            top_up
        );

        bw_manager.try_use_bandwidth(consume).await.unwrap();
        let remaining = top_up - consume;
        assert_eq!(client_bandwidth.available().await, remaining);
        assert_eq!(
            test.peer_manager.query_bandwidth(public_key).await.unwrap(),
            remaining
        );

        test.finish().await
    }
}
