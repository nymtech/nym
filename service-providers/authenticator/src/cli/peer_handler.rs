// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sdk::TaskClient;
use nym_wireguard::peer_controller::{PeerControlRequest, PeerControlResponse};
use tokio::sync::mpsc;

pub struct DummyHandler {
    peer_rx: mpsc::UnboundedReceiver<PeerControlRequest>,
    response_tx: mpsc::UnboundedSender<PeerControlResponse>,
    task_client: TaskClient,
}

impl DummyHandler {
    pub fn new(
        peer_rx: mpsc::UnboundedReceiver<PeerControlRequest>,
        response_tx: mpsc::UnboundedSender<PeerControlResponse>,
        task_client: TaskClient,
    ) -> Self {
        DummyHandler {
            peer_rx,
            response_tx,
            task_client,
        }
    }

    pub async fn run(mut self) {
        while !self.task_client.is_shutdown() {
            tokio::select! {
                msg = self.peer_rx.recv() => {
                    if let Some(msg) = msg {
                        match msg {
                            PeerControlRequest::AddPeer(peer) => {
                                log::info!("[DUMMY] Adding peer {:?}", peer);
                                self.response_tx.send(PeerControlResponse::AddPeer { success: true }).ok();
                            }
                            PeerControlRequest::RemovePeer(key) => {
                                log::info!("[DUMMY] Removing peer {:?}", key);
                                self.response_tx.send(PeerControlResponse::RemovePeer { success: true }).ok();
                            }
                            PeerControlRequest::QueryBandwidth(key) => {
                                log::info!("[DUMMY] Querying bandwidth for peer {:?}", key);
                                self.response_tx.send(PeerControlResponse::QueryBandwidth { bandwidth_data: None }).ok();
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
