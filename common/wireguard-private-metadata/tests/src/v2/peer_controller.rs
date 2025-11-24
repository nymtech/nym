// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// not declared as a 'global' since I can imagine it might change between versions

use nym_wireguard::peer_controller::PeerControlRequest;
use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc::Receiver;

#[derive(Hash, PartialOrd, PartialEq, Clone, Debug, Eq, Copy)]
pub enum PeerControlRequestTypeV2 {
    AddPeer,
    RemovePeer,
    QueryPeer,
    GetClientBandwidthByKey,
    GetClientBandwidthByIp { ip: IpAddr },
    GetVerifierByKey,
    GetVerifierByIp { ip: IpAddr },
}

impl From<&PeerControlRequest> for PeerControlRequestTypeV2 {
    fn from(req: &PeerControlRequest) -> Self {
        match req {
            PeerControlRequest::AddPeer { .. } => PeerControlRequestTypeV2::AddPeer,
            PeerControlRequest::RegisterPeer { .. } => PeerControlRequestTypeV2::AddPeer,
            PeerControlRequest::RemovePeer { .. } => PeerControlRequestTypeV2::RemovePeer,
            PeerControlRequest::QueryPeer { .. } => PeerControlRequestTypeV2::QueryPeer,
            PeerControlRequest::GetClientBandwidthByKey { .. } => {
                PeerControlRequestTypeV2::GetClientBandwidthByKey
            }
            PeerControlRequest::GetClientBandwidthByIp { ip, .. } => {
                PeerControlRequestTypeV2::GetClientBandwidthByIp { ip: *ip }
            }
            PeerControlRequest::GetVerifierByKey { .. } => {
                PeerControlRequestTypeV2::GetVerifierByKey
            }
            PeerControlRequest::GetVerifierByIp { ip, .. } => {
                PeerControlRequestTypeV2::GetVerifierByIp { ip: *ip }
            }
        }
    }
}

// all responses are registered as a queue for particular type
// (this is because the actual type can't be cloned as the `Error` does not implement Clone)
type RegisteredResponses =
    HashMap<PeerControlRequestTypeV2, VecDeque<Box<dyn Any + Send + Sync + 'static>>>;

#[derive(Clone, Default)]
pub struct MockPeerControllerStateV2 {
    registered_responses: Arc<RwLock<RegisteredResponses>>,
}

impl MockPeerControllerStateV2 {
    pub async fn register_response(
        &self,
        request: PeerControlRequestTypeV2,
        response: impl Any + Send + Sync + 'static,
    ) {
        self.registered_responses
            .write()
            .await
            .entry(request)
            .or_default()
            .push_back(Box::new(response));
    }

    pub async fn clear_registered_responses(&self) {
        self.registered_responses.write().await.clear();
    }
}

pub struct MockPeerControllerV2 {
    state: MockPeerControllerStateV2,
    request_rx: Receiver<PeerControlRequest>,
}

impl MockPeerControllerV2 {
    pub(crate) fn new(
        state: MockPeerControllerStateV2,
        request_rx: Receiver<PeerControlRequest>,
    ) -> Self {
        MockPeerControllerV2 { state, request_rx }
    }

    async fn handle_request(&mut self, request: PeerControlRequest) {
        let typ = PeerControlRequestTypeV2::from(&request);

        let mut guard = self.state.registered_responses.write().await;
        let Some(registered_responses) = guard.get_mut(&typ) else {
            panic!(
                "received a request for {typ:?} but there are no registered responses - this is probably due to a bug in your test setup"
            );
        };

        let Some(response) = registered_responses.pop_front() else {
            panic!(
                "received a request for {typ:?} but there are no registered responses - this is probably due to a bug in your test setup"
            );
        };

        match request {
            PeerControlRequest::AddPeer { response_tx, .. } => {
                response_tx
                    .send(
                        *response
                            .downcast()
                            .expect("registered response has mismatched type"),
                    )
                    .unwrap();
            }
            PeerControlRequest::RegisterPeer { response_tx, .. } => {
                response_tx
                    .send(
                        *response
                            .downcast()
                            .expect("registered response has mismatched type"),
                    )
                    .unwrap();
            }
            PeerControlRequest::RemovePeer { response_tx, .. } => {
                response_tx
                    .send(
                        *response
                            .downcast()
                            .expect("registered response has mismatched type"),
                    )
                    .unwrap();
            }
            PeerControlRequest::QueryPeer { response_tx, .. } => {
                response_tx
                    .send(
                        *response
                            .downcast()
                            .expect("registered response has mismatched type"),
                    )
                    .unwrap();
            }
            PeerControlRequest::GetClientBandwidthByKey { response_tx, .. } => {
                response_tx
                    .send(
                        *response
                            .downcast()
                            .expect("registered response has mismatched type"),
                    )
                    .unwrap();
            }
            PeerControlRequest::GetClientBandwidthByIp { response_tx, .. } => {
                response_tx
                    .send(
                        *response
                            .downcast()
                            .expect("registered response has mismatched type"),
                    )
                    .unwrap();
            }
            PeerControlRequest::GetVerifierByKey { response_tx, .. } => {
                response_tx
                    .send(
                        *response
                            .downcast()
                            .expect("registered response has mismatched type"),
                    )
                    .ok();
            }
            PeerControlRequest::GetVerifierByIp { response_tx, .. } => {
                response_tx
                    .send(
                        *response
                            .downcast()
                            .expect("registered response has mismatched type"),
                    )
                    .ok();
            }
        }
    }

    pub(crate) async fn run(&mut self) {
        while let Some(request) = self.request_rx.recv().await {
            self.handle_request(request).await;
        }
    }
}
