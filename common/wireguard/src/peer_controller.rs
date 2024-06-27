// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use defguard_wireguard_rs::{
    host::{Host, Peer},
    key::Key,
    WGApi, WireguardInterfaceApi,
};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::IntervalStream, StreamExt};

use crate::WgApiWrapper;

const DEFAULT_PEER_TIMEOUT: Duration = Duration::from_secs(60 * 60); // 1 hour
const DEFAULT_PEER_TIMEOUT_CHECK: Duration = Duration::from_secs(60); // 1 minute

pub enum PeerControlMessage {
    AddPeer(Peer),
    RemovePeer(Key),
}

pub struct PeerController {
    peer_rx: mpsc::UnboundedReceiver<PeerControlMessage>,
    wg_api: Arc<WgApiWrapper>,
    timeout_check_interval: IntervalStream,
}

impl PeerController {
    pub fn new(
        wg_api: Arc<WgApiWrapper>,
        peer_rx: mpsc::UnboundedReceiver<PeerControlMessage>,
    ) -> Self {
        let timeout_check_interval = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(DEFAULT_PEER_TIMEOUT_CHECK),
        );
        PeerController {
            wg_api,
            peer_rx,
            timeout_check_interval,
        }
    }

    fn remove_stale_peers(wg_api: &WGApi, host: Host) {
        let current_timestamp = SystemTime::now();
        for (key, peer) in host.peers.iter() {
            if let Some(timestamp) = peer.last_handshake {
                if let Ok(duration_since_handshake) = current_timestamp.duration_since(timestamp) {
                    if duration_since_handshake > DEFAULT_PEER_TIMEOUT {
                        if let Err(e) = wg_api.remove_peer(key) {
                            log::error!("Could not remove stale peer: {:?}", e);
                        } else {
                            log::debug!("Removed stale peer {:?}", key);
                        }
                    }
                }
            }
        }
    }

    pub async fn run(&mut self, mut task_client: nym_task::TaskClient) {
        loop {
            tokio::select! {
                _ = self.timeout_check_interval.next() => {
                    match self.wg_api.inner.read_interface_data() {
                        Ok(host) => Self::remove_stale_peers(&self.wg_api.inner, host),
                        Err(e) => { log::error!("Could not read peer data: {:?}", e); },
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
                            }
                        }
                        Some(PeerControlMessage::RemovePeer(peer_pubkey)) => {
                            if let Err(e) = self.wg_api.inner.remove_peer(&peer_pubkey) {
                                log::error!("Could not remove peer: {:?}", e);
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
