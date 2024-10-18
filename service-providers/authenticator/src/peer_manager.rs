// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::*;
use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask};
use futures::channel::oneshot;
use nym_authenticator_requests::latest::registration::{GatewayClient, RemainingBandwidthData};
use nym_wireguard::{
    peer_controller::{
        AddPeerControlResponse, PeerControlRequest, QueryBandwidthControlResponse,
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
    pub async fn add_peer(
        &mut self,
        mut peer: Peer,
        client: &GatewayClient,
        client_id: Option<i64>,
    ) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();
        peer.allowed_ips
            .push(IpAddrMask::new(client.private_ip, 32));
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
        peer_public_key: PeerPublicKey,
    ) -> Result<Option<RemainingBandwidthData>> {
        let key = Key::new(peer_public_key.to_bytes());
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
        } = response_rx
            .await
            .map_err(|_| AuthenticatorError::InternalError("no response for query".to_string()))?;
        if !success {
            return Err(AuthenticatorError::InternalError(
                "querying bandwidth could not be performed".to_string(),
            ));
        }
        Ok(bandwidth_data)
    }
}
