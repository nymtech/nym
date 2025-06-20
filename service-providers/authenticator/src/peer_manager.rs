// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::*;
use defguard_wireguard_rs::{host::Peer, key::Key};
use futures::channel::oneshot;
use nym_credential_verification::ClientBandwidth;
use nym_wireguard::{
    peer_controller::{
        AddPeerControlResponse, GetClientBandwidthControlResponse, PeerControlRequest,
        QueryPeerControlResponse, RemovePeerControlResponse,
    },
    WireguardGatewayData,
};
use nym_wireguard_types::PeerPublicKey;

pub struct PeerManager {
    pub(crate) wireguard_gateway_data: WireguardGatewayData,
}

impl PeerManager {
    pub fn new(wireguard_gateway_data: WireguardGatewayData) -> Self {
        PeerManager {
            wireguard_gateway_data,
        }
    }
    pub async fn add_peer(&mut self, peer: Peer) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::AddPeer { peer, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| AuthenticatorError::PeerInteractionStopped)?;

        let AddPeerControlResponse { success } = response_rx.await.map_err(|_| {
            AuthenticatorError::InternalError("no response for add peer".to_string())
        })?;
        if !success {
            return Err(AuthenticatorError::InternalError(
                "adding peer could not be performed".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn _remove_peer(&mut self, pub_key: PeerPublicKey) -> Result<()> {
        let key = Key::new(pub_key.to_bytes());
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::RemovePeer { key, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| AuthenticatorError::PeerInteractionStopped)?;

        let RemovePeerControlResponse { success } = response_rx.await.map_err(|_| {
            AuthenticatorError::InternalError("no response for remove peer".to_string())
        })?;
        if !success {
            return Err(AuthenticatorError::InternalError(
                "removing peer could not be performed".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn query_peer(&mut self, public_key: PeerPublicKey) -> Result<Option<Peer>> {
        let key = Key::new(public_key.to_bytes());
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::QueryPeer { key, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| AuthenticatorError::PeerInteractionStopped)?;

        let QueryPeerControlResponse { success, peer } = response_rx.await.map_err(|_| {
            AuthenticatorError::InternalError("no response for query peer".to_string())
        })?;
        if !success {
            return Err(AuthenticatorError::InternalError(
                "querying peer could not be performed".to_string(),
            ));
        }
        Ok(peer)
    }

    pub async fn query_bandwidth(&mut self, public_key: PeerPublicKey) -> Result<Option<i64>> {
        let res = if let Some(client_bandwidth) = self.query_client_bandwidth(public_key).await? {
            Some(client_bandwidth.available().await)
        } else {
            None
        };
        Ok(res)
    }

    pub async fn query_client_bandwidth(
        &mut self,
        key: PeerPublicKey,
    ) -> Result<Option<ClientBandwidth>> {
        let key = Key::new(key.to_bytes());
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::GetClientBandwidth { key, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| AuthenticatorError::PeerInteractionStopped)?;

        let GetClientBandwidthControlResponse { client_bandwidth } =
            response_rx.await.map_err(|_| {
                AuthenticatorError::InternalError(
                    "no response for query client bandwidth".to_string(),
                )
            })?;
        Ok(client_bandwidth)
    }
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, sync::Arc};

    use nym_credential_verification::{
        bandwidth_storage_manager::BandwidthStorageManager, ecash::MockEcashManager,
    };
    use nym_credentials_interface::Bandwidth;
    use nym_crypto::asymmetric::x25519::KeyPair;
    use nym_gateway_storage::traits::{mock::MockGatewayStorage, BandwidthGatewayStorage};
    use nym_wireguard::peer_controller::{start_controller, stop_controller};
    use rand::rngs::OsRng;
    use time::{Duration, OffsetDateTime};

    use crate::{config::Authenticator, mixnet_listener::credential_storage_preparation};

    use super::*;

    #[tokio::test]
    async fn add_peer() {
        let (wireguard_data, request_rx) = WireguardGatewayData::new(
            Authenticator::default().into(),
            Arc::new(KeyPair::new(&mut OsRng)),
        );
        let mut peer_manager = PeerManager::new(wireguard_data);
        let (storage, task_manager) = start_controller(
            peer_manager.wireguard_gateway_data.peer_tx().clone(),
            request_rx,
        );
        let peer = Peer::default();
        let ecash_manager = MockEcashManager::new(Box::new(storage.clone()));

        assert!(peer_manager.add_peer(peer.clone()).await.is_err());

        let client_id = storage
            .insert_wireguard_peer(&peer, FromStr::from_str("entry_wireguard").unwrap())
            .await
            .unwrap();
        assert!(peer_manager.add_peer(peer.clone()).await.is_err());

        credential_storage_preparation(Arc::new(ecash_manager), client_id)
            .await
            .unwrap();
        peer_manager.add_peer(peer.clone()).await.unwrap();

        stop_controller(task_manager).await;
    }

    async fn helper_add_peer(storage: &MockGatewayStorage, peer_manager: &mut PeerManager) -> i64 {
        let peer = Peer::default();
        let ecash_manager = MockEcashManager::new(Box::new(storage.clone()));
        let client_id = storage
            .insert_wireguard_peer(&peer, FromStr::from_str("entry_wireguard").unwrap())
            .await
            .unwrap();
        credential_storage_preparation(Arc::new(ecash_manager), client_id)
            .await
            .unwrap();
        peer_manager.add_peer(peer.clone()).await.unwrap();

        client_id
    }

    #[tokio::test]
    async fn remove_peer() {
        let (wireguard_data, request_rx) = WireguardGatewayData::new(
            Authenticator::default().into(),
            Arc::new(KeyPair::new(&mut OsRng)),
        );
        let mut peer_manager = PeerManager::new(wireguard_data);
        let key = Key::default();
        let public_key = PeerPublicKey::from_str(&key.to_string()).unwrap();
        let (storage, task_manager) = start_controller(
            peer_manager.wireguard_gateway_data.peer_tx().clone(),
            request_rx,
        );

        helper_add_peer(&storage, &mut peer_manager).await;
        peer_manager._remove_peer(public_key).await.unwrap();

        stop_controller(task_manager).await;
    }

    #[tokio::test]
    async fn query_peer() {
        let (wireguard_data, request_rx) = WireguardGatewayData::new(
            Authenticator::default().into(),
            Arc::new(KeyPair::new(&mut OsRng)),
        );
        let mut peer_manager = PeerManager::new(wireguard_data);
        let key = Key::default();
        let public_key = PeerPublicKey::from_str(&key.to_string()).unwrap();
        let (storage, task_manager) = start_controller(
            peer_manager.wireguard_gateway_data.peer_tx().clone(),
            request_rx,
        );

        assert!(peer_manager.query_peer(public_key).await.unwrap().is_none());

        helper_add_peer(&storage, &mut peer_manager).await;
        let peer = peer_manager.query_peer(public_key).await.unwrap().unwrap();
        assert_eq!(peer.public_key, key);

        stop_controller(task_manager).await;
    }

    #[tokio::test]
    async fn query_bandwidth() {
        let (wireguard_data, request_rx) = WireguardGatewayData::new(
            Authenticator::default().into(),
            Arc::new(KeyPair::new(&mut OsRng)),
        );
        let mut peer_manager = PeerManager::new(wireguard_data);
        let key = Key::default();
        let public_key = PeerPublicKey::from_str(&key.to_string()).unwrap();
        let (storage, task_manager) = start_controller(
            peer_manager.wireguard_gateway_data.peer_tx().clone(),
            request_rx,
        );

        assert!(peer_manager
            .query_bandwidth(public_key)
            .await
            .unwrap()
            .is_none());

        helper_add_peer(&storage, &mut peer_manager).await;
        let available_bandwidth = peer_manager
            .query_bandwidth(public_key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(available_bandwidth, 0);

        stop_controller(task_manager).await;
    }

    #[tokio::test]
    async fn query_client_bandwidth() {
        let (wireguard_data, request_rx) = WireguardGatewayData::new(
            Authenticator::default().into(),
            Arc::new(KeyPair::new(&mut OsRng)),
        );
        let mut peer_manager = PeerManager::new(wireguard_data);
        let key = Key::default();
        let public_key = PeerPublicKey::from_str(&key.to_string()).unwrap();
        let (storage, task_manager) = start_controller(
            peer_manager.wireguard_gateway_data.peer_tx().clone(),
            request_rx,
        );

        assert!(peer_manager
            .query_client_bandwidth(public_key)
            .await
            .unwrap()
            .is_none());

        helper_add_peer(&storage, &mut peer_manager).await;
        let available_bandwidth = peer_manager
            .query_client_bandwidth(public_key)
            .await
            .unwrap()
            .unwrap()
            .available()
            .await;
        assert_eq!(available_bandwidth, 0);

        stop_controller(task_manager).await;
    }

    #[tokio::test]
    async fn increase_decrease_bandwidth() {
        let (wireguard_data, request_rx) = WireguardGatewayData::new(
            Authenticator::default().into(),
            Arc::new(KeyPair::new(&mut OsRng)),
        );
        let mut peer_manager = PeerManager::new(wireguard_data);
        let key = Key::default();
        let public_key = PeerPublicKey::from_str(&key.to_string()).unwrap();
        let top_up = 42;
        let consume = 4;
        let (storage, task_manager) = start_controller(
            peer_manager.wireguard_gateway_data.peer_tx().clone(),
            request_rx,
        );

        let client_id = helper_add_peer(&storage, &mut peer_manager).await;
        let client_bandwidth = peer_manager
            .query_client_bandwidth(public_key)
            .await
            .unwrap()
            .unwrap();

        let mut bw_manager = BandwidthStorageManager::new(
            Box::new(storage),
            client_bandwidth.clone(),
            client_id,
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
            peer_manager
                .query_bandwidth(public_key)
                .await
                .unwrap()
                .unwrap(),
            top_up
        );

        bw_manager.try_use_bandwidth(consume).await.unwrap();
        let remaining = top_up - consume;
        assert_eq!(client_bandwidth.available().await, remaining);
        assert_eq!(
            peer_manager
                .query_bandwidth(public_key)
                .await
                .unwrap()
                .unwrap(),
            remaining
        );

        stop_controller(task_manager).await;
    }
}
