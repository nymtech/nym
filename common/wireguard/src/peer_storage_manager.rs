// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use defguard_wireguard_rs::host::Peer;
use nym_gateway_storage::models::WireguardPeer;
use nym_gateway_storage::GatewayStorage;
use std::time::Duration;
use time::OffsetDateTime;

const DEFAULT_PEER_MAX_FLUSHING_RATE: Duration = Duration::from_secs(60 * 60 * 24); // 24h
const DEFAULT_PEER_MAX_DELTA_FLUSHING_AMOUNT: u64 = 512 * 1024 * 1024; // 512MB

#[derive(Debug, Clone, Copy)]
pub struct PeerFlushingBehaviourConfig {
    /// Defines maximum delay between peer information being flushed to the persistent storage.
    pub peer_max_flushing_rate: Duration,

    /// Defines a maximum change in peer before it gets flushed to the persistent storage.
    pub peer_max_delta_flushing_amount: u64,
}

impl Default for PeerFlushingBehaviourConfig {
    fn default() -> Self {
        Self {
            peer_max_flushing_rate: DEFAULT_PEER_MAX_FLUSHING_RATE,
            peer_max_delta_flushing_amount: DEFAULT_PEER_MAX_DELTA_FLUSHING_AMOUNT,
        }
    }
}

pub struct PeerStorageManager {
    pub(crate) storage: GatewayStorage,
    pub(crate) peer_information: Option<PeerInformation>,
    pub(crate) cfg: PeerFlushingBehaviourConfig,
    pub(crate) with_client_id: bool,
}

impl PeerStorageManager {
    pub(crate) fn new(storage: GatewayStorage, peer: Peer, with_client_id: bool) -> Self {
        let peer_information = Some(PeerInformation::new(peer));
        Self {
            storage,
            peer_information,
            cfg: PeerFlushingBehaviourConfig::default(),
            with_client_id,
        }
    }

    pub(crate) fn get_peer(&self) -> Option<WireguardPeer> {
        self.peer_information
            .as_ref()
            .map(|p| p.peer.clone().into())
    }

    pub(crate) fn remove_peer(&mut self) {
        self.peer_information = None;
    }

    pub(crate) fn update_trx(&mut self, kernel_peer: &Peer) {
        if let Some(peer_information) = self.peer_information.as_mut() {
            peer_information.update_trx_bytes(kernel_peer.tx_bytes, kernel_peer.rx_bytes);
        }
    }

    pub(crate) async fn sync_storage_peer(&mut self) -> Result<(), Error> {
        let Some(peer_information) = self.peer_information.as_mut() else {
            return Ok(());
        };
        if !peer_information.should_sync(self.cfg) {
            return Ok(());
        }
        if self
            .storage
            .get_wireguard_peer(&peer_information.peer().public_key.to_string())
            .await?
            .is_none()
        {
            self.peer_information = None;
            return Ok(());
        }
        self.storage
            .insert_wireguard_peer(peer_information.peer(), self.with_client_id)
            .await?;

        peer_information.resync_peer_with_storage();

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PeerInformation {
    pub(crate) peer: Peer,
    pub(crate) last_synced: OffsetDateTime,

    pub(crate) bytes_delta_since_sync: u64,
}

impl PeerInformation {
    pub fn new(peer: Peer) -> PeerInformation {
        PeerInformation {
            peer,
            last_synced: OffsetDateTime::now_utc(),
            bytes_delta_since_sync: 0,
        }
    }

    pub(crate) fn should_sync(&self, cfg: PeerFlushingBehaviourConfig) -> bool {
        if self.bytes_delta_since_sync >= cfg.peer_max_delta_flushing_amount {
            return true;
        }

        if self.last_synced + cfg.peer_max_flushing_rate < OffsetDateTime::now_utc()
            && self.bytes_delta_since_sync != 0
        {
            return true;
        }

        false
    }

    pub(crate) fn peer(&self) -> &Peer {
        &self.peer
    }

    pub(crate) fn update_trx_bytes(&mut self, tx_bytes: u64, rx_bytes: u64) {
        self.bytes_delta_since_sync += tx_bytes.saturating_sub(self.peer.tx_bytes)
            + rx_bytes.saturating_sub(self.peer.rx_bytes);
        self.peer.tx_bytes = tx_bytes;
        self.peer.rx_bytes = rx_bytes;
    }

    pub(crate) fn resync_peer_with_storage(&mut self) {
        self.bytes_delta_since_sync = 0;
        self.last_synced = OffsetDateTime::now_utc();
    }
}
