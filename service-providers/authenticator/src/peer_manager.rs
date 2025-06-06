// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::*;
use defguard_wireguard_rs::{host::Peer, key::Key};
use futures::channel::oneshot;
use nym_authenticator_requests::{
    latest::registration::{GatewayClient, RemainingBandwidthData},
    traits::QueryBandwidthMessage,
};
use nym_credential_verification::ClientBandwidth;
use nym_wireguard::{
    peer_controller::{
        AddPeerControlResponse, GetClientBandwidthControlResponse, PeerControlRequest,
        QueryBandwidthControlResponse, QueryPeerControlResponse, RemovePeerControlResponse,
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
    pub async fn add_peer(&mut self, peer: Peer, client_id: Option<i64>) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::AddPeer {
            peer,
            client_id,
            response_tx,
        };
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

    pub async fn _remove_peer(&mut self, client: &GatewayClient) -> Result<()> {
        let key = Key::new(client.pub_key().to_bytes());
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::RemovePeer { key, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| AuthenticatorError::PeerInteractionStopped)?;

        let RemovePeerControlResponse { success } = response_rx.await.map_err(|_| {
            AuthenticatorError::InternalError("no response for add peer".to_string())
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

    pub async fn query_bandwidth(
        &mut self,
        msg: Box<dyn QueryBandwidthMessage + Send + Sync + 'static>,
    ) -> Result<Option<RemainingBandwidthData>> {
        let key = Key::new(msg.pub_key().to_bytes());
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::QueryBandwidth { key, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| AuthenticatorError::PeerInteractionStopped)?;

        let QueryBandwidthControlResponse {
            success,
            bandwidth_data,
        } = response_rx.await.map_err(|_| {
            AuthenticatorError::InternalError("no response for query bandwidth".to_string())
        })?;
        if !success {
            return Err(AuthenticatorError::InternalError(
                "querying bandwidth could not be performed".to_string(),
            ));
        }
        Ok(bandwidth_data)
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
