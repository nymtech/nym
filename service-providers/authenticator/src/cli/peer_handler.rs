// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sdk::TaskClient;
use nym_wireguard::peer_controller::{
    AddPeerControlResponse, GetClientBandwidthControlResponse, PeerControlRequest,
    QueryBandwidthControlResponse, QueryPeerControlResponse, RemovePeerControlResponse,
};
use tokio::sync::mpsc;

pub struct DummyHandler {
    peer_rx: mpsc::Receiver<PeerControlRequest>,
    task_client: TaskClient,
}

impl DummyHandler {
    pub fn new(peer_rx: mpsc::Receiver<PeerControlRequest>, task_client: TaskClient) -> Self {
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
                            PeerControlRequest::AddPeer { peer, client_id, response_tx } => {
                                log::info!("[DUMMY] Adding peer {peer:?} with client id {client_id:?}");
                                response_tx.send(AddPeerControlResponse { success: true }).ok();
                            }
                            PeerControlRequest::RemovePeer { key, response_tx } => {
                                log::info!("[DUMMY] Removing peer {key:?}");
                                response_tx.send(RemovePeerControlResponse { success: true }).ok();
                            }
                            PeerControlRequest::QueryPeer{key, response_tx} => {
                                log::info!("[DUMMY] Querying peer {key:?}");
                                response_tx.send(QueryPeerControlResponse { success: true, peer: None }).ok();
                            }
                            PeerControlRequest::QueryBandwidth{key, response_tx} => {
                                log::info!("[DUMMY] Querying bandwidth for peer {key:?}");
                                response_tx.send(QueryBandwidthControlResponse { success: true, bandwidth_data: None }).ok();
                            }
                            PeerControlRequest::GetClientBandwidth{key, response_tx} => {
                                log::info!("[DUMMY] Getting client bandwidth for peer {key:?}");
                                response_tx.send(GetClientBandwidthControlResponse {client_bandwidth: None }).ok();
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
