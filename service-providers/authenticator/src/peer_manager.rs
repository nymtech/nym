// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::*;
use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask};
use nym_wireguard::{
    peer_controller::{PeerControlRequest, PeerControlResponse},
    WireguardGatewayData,
};
use nym_wireguard_types::{registration::RemainingBandwidthData, GatewayClient, PeerPublicKey};
use tokio::sync::mpsc::UnboundedReceiver;

pub struct PeerManager {
    pub(crate) wireguard_gateway_data: WireguardGatewayData,

    pub(crate) response_rx: UnboundedReceiver<PeerControlResponse>,
}

impl PeerManager {
    pub fn new(
        wireguard_gateway_data: WireguardGatewayData,
        response_rx: UnboundedReceiver<PeerControlResponse>,
    ) -> Self {
        PeerManager {
            wireguard_gateway_data,
            response_rx,
        }
    }
    pub async fn add_peer(&mut self, client: &GatewayClient) -> Result<()> {
        let mut peer = Peer::new(Key::new(client.pub_key.to_bytes()));
        peer.allowed_ips
            .push(IpAddrMask::new(client.private_ip, 32));
        let msg = PeerControlRequest::AddPeer(peer);
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .map_err(|_| AuthenticatorError::PeerInteractionStopped)?;

        let PeerControlResponse::AddPeer { success } =
            self.response_rx
                .recv()
                .await
                .ok_or(AuthenticatorError::InternalError(
                    "no response for add peer".to_string(),
                ))?
        else {
            return Err(AuthenticatorError::InternalError(
                "unexpected response type".to_string(),
            ));
        };
        if !success {
            return Err(AuthenticatorError::InternalError(
                "adding peer could not be performed".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn _remove_peer(&mut self, client: &GatewayClient) -> Result<()> {
        let key = Key::new(client.pub_key().to_bytes());
        let msg = PeerControlRequest::RemovePeer(key);
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .map_err(|_| AuthenticatorError::PeerInteractionStopped)?;

        let PeerControlResponse::RemovePeer { success } =
            self.response_rx
                .recv()
                .await
                .ok_or(AuthenticatorError::InternalError(
                    "no response for add peer".to_string(),
                ))?
        else {
            return Err(AuthenticatorError::InternalError(
                "unexpected response type".to_string(),
            ));
        };
        if !success {
            return Err(AuthenticatorError::InternalError(
                "adding peer could not be performed".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn query_peer(&mut self, public_key: PeerPublicKey) -> Result<Option<Peer>> {
        let key = Key::new(public_key.to_bytes());
        let msg = PeerControlRequest::QueryPeer(key);
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .map_err(|_| AuthenticatorError::PeerInteractionStopped)?;

        let PeerControlResponse::QueryPeer { success, peer } = self
            .response_rx
            .recv()
            .await
            .ok_or(AuthenticatorError::InternalError(
                "no response for query peer".to_string(),
            ))?
        else {
            return Err(AuthenticatorError::InternalError(
                "unexpected response type".to_string(),
            ));
        };
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
        let msg = PeerControlRequest::QueryBandwidth(key);
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .map_err(|_| AuthenticatorError::PeerInteractionStopped)?;

        let PeerControlResponse::QueryBandwidth { bandwidth_data } = self
            .response_rx
            .recv()
            .await
            .ok_or(AuthenticatorError::InternalError(
                "no response for query".to_string(),
            ))?
        else {
            return Err(AuthenticatorError::InternalError(
                "unexpected response type".to_string(),
            ));
        };
        Ok(bandwidth_data)
    }
}
