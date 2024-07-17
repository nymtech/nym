// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use chrono::{Timelike, Utc};
use defguard_wireguard_rs::{host::Peer, key::Key, WireguardInterfaceApi};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::IntervalStream, StreamExt};

use crate::error::Error;
use crate::WgApiWrapper;

const DEFAULT_PEER_TIMEOUT_CHECK: Duration = Duration::from_secs(60); // 1 minute
const BANDWIDTH_CAP_PER_DAY: u64 = 1024 * 1024 * 1024; // 1 GB

pub enum PeerControlMessage {
    AddPeer(Peer),
    RemovePeer(Key),
}

pub struct PeerController {
    peer_rx: mpsc::UnboundedReceiver<PeerControlMessage>,
    wg_api: Arc<WgApiWrapper>,
    timeout_check_interval: IntervalStream,
    active_peers: HashMap<Key, Peer>,
    suspended_peers: HashMap<Key, Peer>,
}

impl PeerController {
    pub fn new(
        wg_api: Arc<WgApiWrapper>,
        peers: Vec<Peer>,
        peer_rx: mpsc::UnboundedReceiver<PeerControlMessage>,
    ) -> Self {
        let timeout_check_interval = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(DEFAULT_PEER_TIMEOUT_CHECK),
        );
        let active_peers = peers
            .into_iter()
            .map(|peer| (peer.public_key.clone(), peer))
            .collect();

        PeerController {
            wg_api,
            peer_rx,
            timeout_check_interval,
            active_peers,
            suspended_peers: HashMap::new(),
        }
    }

    fn check_peers(&mut self) -> Result<(), Error> {
        // Add 1 second to cover edge cases. At worst, we give one second worth of bandwidth
        // by resetting the bandwidth twice
        let reset = Utc::now().num_seconds_from_midnight() as u64
            <= DEFAULT_PEER_TIMEOUT_CHECK.as_secs() + 1;

        if reset {
            for (_, peer) in self.suspended_peers.drain() {
                self.wg_api.inner.configure_peer(&peer)?;
            }
        }
        let host = self.wg_api.inner.read_interface_data()?;
        if reset {
            self.active_peers = host.peers;
        } else {
            for (key, peer) in host.peers.iter() {
                let prev_peer = self.active_peers.get(key).ok_or(Error::PeerMismatch)?;
                let data_usage = (peer.rx_bytes + peer.tx_bytes)
                    .saturating_sub(prev_peer.rx_bytes + prev_peer.tx_bytes);
                if data_usage > BANDWIDTH_CAP_PER_DAY {
                    self.wg_api.inner.remove_peer(key)?;
                    let (moved_key, moved_peer) = self
                        .active_peers
                        .remove_entry(key)
                        .ok_or(Error::PeerMismatch)?;
                    self.suspended_peers.insert(moved_key, moved_peer);
                }
            }
        }

        Ok(())
    }

    pub async fn run(&mut self, mut task_client: nym_task::TaskClient) {
        loop {
            tokio::select! {
                _ = self.timeout_check_interval.next() => {
                    if let Err(e) = self.check_peers() {
                        log::error!("Error while periodically checking peers: {:?}", e);
                    }
                }
                _ = task_client.recv() => {
                    log::trace!("PeerController handler: Received shutdown");
                    break;
                }
                msg = self.peer_rx.recv() => {
                    match msg {
                        Some(PeerControlMessage::AddPeer(peer)) => {
                            if let Err(e) = self.wg_api.inner.configure_peer(&peer) {
                                log::error!("Could not configure peer: {:?}", e);
                            } else {
                                self.active_peers.insert(peer.public_key.clone(), peer);
                            }
                        }
                        Some(PeerControlMessage::RemovePeer(peer_pubkey)) => {
                            if let Err(e) = self.wg_api.inner.remove_peer(&peer_pubkey) {
                                log::error!("Could not remove peer: {:?}", e);
                            } else {
                                self.active_peers.remove(&peer_pubkey);
                                self.suspended_peers.remove(&peer_pubkey);
                            }
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
