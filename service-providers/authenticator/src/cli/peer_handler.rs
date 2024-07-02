// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sdk::TaskClient;
use nym_wireguard::peer_controller::PeerControlMessage;
use tokio::sync::mpsc;

pub struct DummyHandler {
    peer_rx: mpsc::UnboundedReceiver<PeerControlMessage>,
    task_client: TaskClient,
}

impl DummyHandler {
    pub fn new(
        peer_rx: mpsc::UnboundedReceiver<PeerControlMessage>,
        task_client: TaskClient,
    ) -> Self {
        DummyHandler {
            peer_rx,
            task_client,
        }
    }

    pub async fn run(mut self) {
        while !self.task_client.is_shutdown() {
            tokio::select! {
                msg = self.peer_rx.recv() => {
                    if let Some(msg) = msg {
                        match msg {
                            PeerControlMessage::AddPeer(peer) => {
                                log::info!("[DUMMY] Adding peer {:?}", peer);
                            }
                            PeerControlMessage::RemovePeer(key) => {
                                log::info!("[DUMMY] Removing peer {:?}", key);
                            }
                        }
                    } else {
                        break;
                    }
                }

                _ = self.task_client.recv() => {
                    log::trace!("DummyHandler: Received shutdown");
                }
            }
        }
        log::debug!("DummyHandler: Exiting");
    }
}
