// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use defguard_wireguard_rs::{host::Peer, key::Key, WireguardInterfaceApi};
use futures::{channel::mpsc, StreamExt};

use crate::WgApiWrapper;

pub enum PeerControlMessage {
    AddPeer(Peer),
    RemovePeer(Key),
}

pub struct PeerController {
    client_rx: mpsc::UnboundedReceiver<PeerControlMessage>,
    wg_api: Arc<WgApiWrapper>,
}

impl PeerController {
    pub fn new(wg_api: Arc<WgApiWrapper>) -> (Self, mpsc::UnboundedSender<PeerControlMessage>) {
        let (client_tx, client_rx) = mpsc::unbounded();

        (PeerController { client_rx, wg_api }, client_tx)
    }

    pub async fn run(&mut self, mut task_client: nym_task::TaskClient) {
        loop {
            tokio::select! {
                _ = task_client.recv() => {
                    log::trace!("PeerController handler: Received shutdown");
                    break;
                }
                msg = self.client_rx.next() => {
                    match msg {
                        Some(PeerControlMessage::AddPeer(peer)) => {
                            if self.wg_api.inner.configure_peer(&peer).is_err() {
                                log::error!("Could not configure peer {:?}", peer);
                            }
                        }
                        Some(PeerControlMessage::RemovePeer(peer_pubkey)) => {
                            if self.wg_api.inner.remove_peer(&peer_pubkey).is_err() {
                                log::error!("Could not remove peer with key {:?}", peer_pubkey);
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
