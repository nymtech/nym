// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use defguard_wireguard_rs::{host::Peer, key::Key, WireguardInterfaceApi};
use tokio::sync::mpsc;

use crate::WgApiWrapper;

pub enum PeerControlMessage {
    AddPeer(Peer),
    RemovePeer(Key),
}

pub struct PeerController {
    peer_rx: mpsc::UnboundedReceiver<PeerControlMessage>,
    wg_api: Arc<WgApiWrapper>,
}

impl PeerController {
    pub fn new(
        wg_api: Arc<WgApiWrapper>,
        peer_rx: mpsc::UnboundedReceiver<PeerControlMessage>,
    ) -> Self {
        PeerController { wg_api, peer_rx }
    }

    pub async fn run(&mut self, mut task_client: nym_task::TaskClient) {
        loop {
            tokio::select! {
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
