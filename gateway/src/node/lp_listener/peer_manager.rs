// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::GatewayError;
use defguard_wireguard_rs::host::Peer;
use defguard_wireguard_rs::key::Key;
use futures::channel::oneshot;
use nym_credential_verification::ClientBandwidth;
use nym_wireguard::peer_controller::IpPair;
use nym_wireguard::{PeerControlRequest, PeerRegistrationData, WireguardGatewayData};
use nym_wireguard_types::PeerPublicKey;
use tracing::error;

/// attempts to replicate [`crate::node::internal_service_providers::authenticator::peer_manager::PeerManager`]
// TODO: put those in the shared crate
pub struct PeerManager {
    pub(crate) wireguard_gateway_data: WireguardGatewayData,
}

impl PeerManager {
    pub fn new(wireguard_gateway_data: WireguardGatewayData) -> Self {
        PeerManager {
            wireguard_gateway_data,
        }
    }

    pub async fn register_peer(
        &self,
        registration_data: PeerRegistrationData,
    ) -> Result<IpPair, GatewayError> {
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::RegisterPeer {
            registration_data,
            response_tx,
        };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|e| {
                GatewayError::InternalError(format!("Failed to send IP allocation request: {e}"))
            })?;

        response_rx
            .await
            .map_err(|e| {
                GatewayError::InternalError(format!("Failed to receive IP allocation: {e}"))
            })?
            .map_err(|e| {
                error!("Failed to allocate IPs from pool: {e}");
                GatewayError::InternalError(format!("Failed to allocate IPs: {e}"))
            })
    }

    pub async fn add_peer(&self, peer: Peer) -> Result<(), GatewayError> {
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::AddPeer { peer, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|e| {
                GatewayError::InternalError(format!("Failed to send peer request: {e}"))
            })?;

        response_rx
            .await
            .map_err(|_| GatewayError::InternalError("no response for add peer".to_string()))?
            .map_err(|err| {
                GatewayError::InternalError(format!("adding peer could not be performed: {err:?}"))
            })
    }

    pub async fn query_peer(
        &self,
        public_key: PeerPublicKey,
    ) -> Result<Option<Peer>, GatewayError> {
        let key = Key::new(public_key.to_bytes());
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::QueryPeer { key, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| {
                GatewayError::InternalError("Failed to send peer query request".to_string())
            })?;

        response_rx
            .await
            .map_err(|_| GatewayError::InternalError("no response for query peer".to_string()))?
            .map_err(|err| {
                GatewayError::InternalError(format!(
                    "querying peer could not be performed: {err:?}"
                ))
            })
    }

    pub async fn query_client_bandwidth(
        &self,
        key: PeerPublicKey,
    ) -> Result<ClientBandwidth, GatewayError> {
        let key = Key::new(key.to_bytes());
        let (response_tx, response_rx) = oneshot::channel();
        let msg = PeerControlRequest::GetClientBandwidthByKey { key, response_tx };
        self.wireguard_gateway_data
            .peer_tx()
            .send(msg)
            .await
            .map_err(|_| {
                GatewayError::InternalError(
                    "Failed to send peer bandwidth query request".to_string(),
                )
            })?;

        response_rx
            .await
            .map_err(|_| {
                GatewayError::InternalError("no response for query peer bandwidth".to_string())
            })?
            .map_err(|err| {
                GatewayError::InternalError(format!(
                    "querying client bandwidth could not be performed: {err:?}"
                ))
            })
    }
}
