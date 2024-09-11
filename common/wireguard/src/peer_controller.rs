// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use chrono::{Timelike, Utc};
use defguard_wireguard_rs::{host::Peer, key::Key, WireguardInterfaceApi};
use nym_gateway_storage::Storage;
use nym_wireguard_types::registration::{RemainingBandwidthData, BANDWIDTH_CAP_PER_DAY};
use std::time::SystemTime;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::IntervalStream, StreamExt};

use crate::error::Error;
use crate::WgApiWrapper;

// To avoid any problems, keep this stale check time bigger (>2x) then the bandwidth cap
// reset time (currently that one is 24h, at UTC midnight)
const DEFAULT_PEER_TIMEOUT: Duration = Duration::from_secs(60 * 60 * 24 * 3); // 3 days
const DEFAULT_PEER_TIMEOUT_CHECK: Duration = Duration::from_secs(60); // 1 minute

pub enum PeerControlRequest {
    AddPeer(Peer),
    RemovePeer(Key),
    QueryPeer(Key),
    QueryBandwidth(Key),
}

pub enum PeerControlResponse {
    AddPeer {
        success: bool,
    },
    RemovePeer {
        success: bool,
    },
    QueryPeer {
        success: bool,
        peer: Option<Peer>,
    },
    QueryBandwidth {
        bandwidth_data: Option<RemainingBandwidthData>,
    },
}

pub struct PeerController<St: Storage> {
    storage: St,
    request_rx: mpsc::UnboundedReceiver<PeerControlRequest>,
    response_tx: mpsc::UnboundedSender<PeerControlResponse>,
    wg_api: Arc<WgApiWrapper>,
    timeout_check_interval: IntervalStream,
    active_peers: HashMap<Key, Peer>,
    suspended_peers: HashMap<Key, Peer>,
    last_seen_bandwidth: HashMap<Key, u64>,
}

impl<St: Storage> PeerController<St> {
    pub fn new(
        storage: St,
        wg_api: Arc<WgApiWrapper>,
        peers: Vec<Peer>,
        suspended_peers: Vec<Peer>,
        request_rx: mpsc::UnboundedReceiver<PeerControlRequest>,
        response_tx: mpsc::UnboundedSender<PeerControlResponse>,
    ) -> Self {
        let timeout_check_interval = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(DEFAULT_PEER_TIMEOUT_CHECK),
        );
        let active_peers: HashMap<Key, Peer> = peers
            .into_iter()
            .map(|peer| (peer.public_key.clone(), peer))
            .collect();
        let suspended_peers: HashMap<Key, Peer> = suspended_peers
            .into_iter()
            .map(|peer| (peer.public_key.clone(), peer))
            .collect();
        let last_seen_bandwidth = active_peers
            .iter()
            .map(|(k, p)| (k.clone(), p.rx_bytes + p.tx_bytes))
            .chain(suspended_peers.keys().map(|k| (k.clone(), 0)))
            .collect();

        PeerController {
            storage,
            wg_api,
            request_rx,
            response_tx,
            timeout_check_interval,
            active_peers,
            suspended_peers,
            last_seen_bandwidth,
        }
    }

    async fn check_stale_peer(
        &self,
        peer: &Peer,
        current_timestamp: SystemTime,
    ) -> Result<bool, Error> {
        if let Some(timestamp) = peer.last_handshake {
            if let Ok(duration_since_handshake) = current_timestamp.duration_since(timestamp) {
                if duration_since_handshake > DEFAULT_PEER_TIMEOUT {
                    self.storage
                        .remove_wireguard_peer(&peer.public_key.to_string())
                        .await?;
                    self.wg_api.inner.remove_peer(&peer.public_key)?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    async fn check_suspend_peer(&mut self, peer: &Peer) -> Result<(), Error> {
        let prev_peer = self
            .active_peers
            .get(&peer.public_key)
            .ok_or(Error::PeerMismatch)?;
        let data_usage =
            (peer.rx_bytes + peer.tx_bytes).saturating_sub(prev_peer.rx_bytes + prev_peer.tx_bytes);
        if data_usage > BANDWIDTH_CAP_PER_DAY {
            self.storage.insert_wireguard_peer(peer, true).await?;
            self.wg_api.inner.remove_peer(&peer.public_key)?;
            self.active_peers
                .remove_entry(&peer.public_key)
                .ok_or(Error::PeerMismatch)?;
            self.suspended_peers
                .insert(peer.public_key.clone(), peer.clone());
        } else {
            // Update peer stored data
            self.storage.insert_wireguard_peer(peer, false).await?;
        }
        Ok(())
    }

    async fn check_peers(&mut self) -> Result<(), Error> {
        // Add 10 seconds to cover edge cases. At worst, we give ten free seconds worth of bandwidth
        // by resetting the bandwidth twice
        let reset = Utc::now().num_seconds_from_midnight() as u64
            <= DEFAULT_PEER_TIMEOUT_CHECK.as_secs() + 10;

        if reset {
            for (_, peer) in self.suspended_peers.drain() {
                self.wg_api.inner.configure_peer(&peer)?;
            }
        }
        let host = self.wg_api.inner.read_interface_data()?;
        self.last_seen_bandwidth = host
            .peers
            .iter()
            .map(|(key, peer)| (key.clone(), peer.rx_bytes + peer.tx_bytes))
            .collect();
        if reset {
            self.active_peers = host.peers;
            for peer in self.active_peers.values() {
                self.storage.insert_wireguard_peer(peer, false).await?;
            }
        } else {
            let peers = self
                .storage
                .get_all_wireguard_peers()
                .await?
                .into_iter()
                .map(Peer::try_from)
                .collect::<Result<Vec<_>, _>>()?;
            let current_timestamp = SystemTime::now();
            for peer in peers {
                if !self.check_stale_peer(&peer, current_timestamp).await? {
                    self.check_suspend_peer(&peer).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn run(&mut self, mut task_client: nym_task::TaskClient) {
        loop {
            tokio::select! {
                _ = self.timeout_check_interval.next() => {
                    if let Err(e) = self.check_peers().await {
                        log::error!("Error while periodically checking peers: {:?}", e);
                    }
                }
                _ = task_client.recv() => {
                    log::trace!("PeerController handler: Received shutdown");
                    break;
                }
                msg = self.request_rx.recv() => {
                    match msg {
                        Some(PeerControlRequest::AddPeer(peer)) => {
                            if let Err(e) = self.storage.insert_wireguard_peer(&peer, false).await {
                                log::error!("Could not insert peer into storage: {:?}", e);
                                self.response_tx.send(PeerControlResponse::AddPeer { success: false }).ok();
                                continue;
                            }
                            let success = if let Err(e) = self.wg_api.inner.configure_peer(&peer) {
                                log::error!("Could not configure peer: {:?}", e);
                                false
                            } else {
                                self.last_seen_bandwidth.insert(peer.public_key.clone(), peer.rx_bytes + peer.tx_bytes);
                                self.active_peers.insert(peer.public_key.clone(), peer);
                                true
                            };
                            self.response_tx.send(PeerControlResponse::AddPeer { success }).ok();
                        }
                        Some(PeerControlRequest::RemovePeer(peer_pubkey)) => {
                            if let Err(e) = self.storage.remove_wireguard_peer(&peer_pubkey.to_string()).await {
                                log::error!("Could not remove peer from storage: {:?}", e);
                                self.response_tx.send(PeerControlResponse::RemovePeer { success: false }).ok();
                                continue;
                            }
                            let success = if let Err(e) = self.wg_api.inner.remove_peer(&peer_pubkey) {
                                log::error!("Could not remove peer: {:?}", e);
                                false
                            } else {
                                self.active_peers.remove(&peer_pubkey);
                                self.suspended_peers.remove(&peer_pubkey);
                                true
                            };
                            self.response_tx.send(PeerControlResponse::RemovePeer { success }).ok();
                        }
                        Some(PeerControlRequest::QueryPeer(peer_pubkey)) => {
                            let (success, peer) = match self.storage.get_wireguard_peer(&peer_pubkey.to_string()).await {
                                Err(e) => {
                                    log::error!("Could not query peer storage {e}");
                                    (false, None)
                                },
                                Ok(None) => (true, None),
                                Ok(Some(storage_peer)) => {
                                    match Peer::try_from(storage_peer) {
                                        Ok(peer) => (true, Some(peer)),
                                        Err(e) => {
                                            log::error!("Could not parse storage peer {e}");
                                            (false, None)
                                        }
                                    }
                                },
                            };
                            self.response_tx.send(PeerControlResponse::QueryPeer { success, peer }).ok();
                        }
                        Some(PeerControlRequest::QueryBandwidth(peer_pubkey)) => {
                            let msg = if self.suspended_peers.contains_key(&peer_pubkey) {
                                PeerControlResponse::QueryBandwidth { bandwidth_data: Some(RemainingBandwidthData{ available_bandwidth: 0, suspended: true }) }
                            } else if let Some(&consumed_bandwidth) = self.last_seen_bandwidth.get(&peer_pubkey) {
                                PeerControlResponse::QueryBandwidth { bandwidth_data: Some(RemainingBandwidthData{ available_bandwidth: BANDWIDTH_CAP_PER_DAY - consumed_bandwidth, suspended: false })}
                            } else {
                                PeerControlResponse::QueryBandwidth { bandwidth_data: None }
                            };
                            self.response_tx.send(msg).ok();
                        }
                        None => {
                            log::trace!("PeerController [main loop]: stopping since channel closed");
                            break;

                        }
                    }
                }
            }
        }
    }
}
