// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in test/mock code
#![allow(clippy::panic)]
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use crate::PeerControlRequest;
use futures::channel::oneshot;
use nym_crypto::asymmetric::x25519;
use std::any::Any;
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::net::IpAddr;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc::Receiver;

pub use defguard_wireguard_rs::key::Key;

pub fn mock_peer_controller(
    request_rx: Receiver<PeerControlRequest>,
) -> (MockPeerController, MockPeerControllerState) {
    let state = MockPeerControllerState::default();

    (
        MockPeerController {
            state: state.clone(),
            request_rx,
        },
        state,
    )
}

// we need `PartialOrd` for being able to store registered responses in the map
// (even though it's not technically the "correct" implementation, for the purposes
// of tests/mocks it's sufficient)
#[derive(Hash, PartialEq, Clone, Debug, Eq)]
pub struct KeyWrapper(Key);

impl PartialOrd for KeyWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.as_slice().partial_cmp(other.0.as_slice())
    }
}

impl Deref for KeyWrapper {
    type Target = Key;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Key> for KeyWrapper {
    fn from(k: Key) -> Self {
        KeyWrapper(k)
    }
}

impl From<&Key> for KeyWrapper {
    fn from(k: &Key) -> Self {
        KeyWrapper(k.clone())
    }
}

#[derive(Hash, PartialOrd, PartialEq, Clone, Debug, Eq)]
pub enum PeerControlRequestType {
    AddPeer { public_key: KeyWrapper },
    RegisterPeer { public_key: KeyWrapper },
    RemovePeer { key: KeyWrapper },
    QueryPeer { key: KeyWrapper },
    GetClientBandwidthByKey { key: KeyWrapper },
    GetClientBandwidthByIp { ip: IpAddr },
    GetVerifierByKey { key: KeyWrapper },
    GetVerifierByIp { ip: IpAddr },
}

impl PeerControlRequestType {
    pub fn peer_key(&self) -> Option<KeyWrapper> {
        match self {
            PeerControlRequestType::AddPeer { public_key } => Some(public_key.clone()),
            PeerControlRequestType::RegisterPeer { public_key } => Some(public_key.clone()),
            PeerControlRequestType::RemovePeer { key } => Some(key.clone()),
            PeerControlRequestType::QueryPeer { key } => Some(key.clone()),
            PeerControlRequestType::GetClientBandwidthByKey { key } => Some(key.clone()),
            PeerControlRequestType::GetClientBandwidthByIp { .. } => None,
            PeerControlRequestType::GetVerifierByKey { key } => Some(key.clone()),
            PeerControlRequestType::GetVerifierByIp { .. } => None,
        }
    }

    pub fn peer_key_unchecked(&self) -> KeyWrapper {
        self.peer_key().expect("this request does not use peer key")
    }
}

impl From<&PeerControlRequest> for PeerControlRequestType {
    fn from(req: &PeerControlRequest) -> Self {
        match req {
            PeerControlRequest::AddPeer { peer, .. } => PeerControlRequestType::AddPeer {
                public_key: (&peer.public_key).into(),
            },
            PeerControlRequest::RegisterPeer {
                registration_data, ..
            } => PeerControlRequestType::RegisterPeer {
                public_key: (&registration_data.public_key).into(),
            },
            PeerControlRequest::RemovePeer { key, .. } => {
                PeerControlRequestType::RemovePeer { key: key.into() }
            }
            PeerControlRequest::QueryPeer { key, .. } => {
                PeerControlRequestType::QueryPeer { key: key.into() }
            }
            PeerControlRequest::GetClientBandwidthByKey { key, .. } => {
                PeerControlRequestType::GetClientBandwidthByKey { key: key.into() }
            }
            PeerControlRequest::GetClientBandwidthByIp { ip, .. } => {
                PeerControlRequestType::GetClientBandwidthByIp { ip: *ip }
            }
            PeerControlRequest::GetVerifierByKey { key, .. } => {
                PeerControlRequestType::GetVerifierByKey { key: key.into() }
            }
            PeerControlRequest::GetVerifierByIp { ip, .. } => {
                PeerControlRequestType::GetVerifierByIp { ip: *ip }
            }
        }
    }
}

pub struct RegisteredResponse {
    // need an additional flag to trigger internal state updates for checking test invariants
    pub success: bool,
    pub content: Box<dyn Any + Send + Sync + 'static>,
}

impl<T, E> From<Result<T, E>> for RegisteredResponse
where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn from(r: Result<T, E>) -> Self {
        let success = r.is_ok();

        RegisteredResponse {
            success,
            content: Box::new(r),
        }
    }
}

// all responses are registered as a queue for particular type
// (this is because the actual type can't be cloned as the `Error` does not implement Clone)
type RegisteredResponses = HashMap<PeerControlRequestType, VecDeque<RegisteredResponse>>;

#[derive(Clone, Default)]
pub struct MockPeerControllerState {
    pub(crate) registered_responses: Arc<RwLock<RegisteredResponses>>,

    // additional state for inspecting during testing
    pub peers: Arc<RwLock<PeersState>>,
}

#[derive(Clone, Default)]
pub struct PeerState {
    /// Has IpPair been allocated to the peer?
    pub register_success: bool,

    // in the future maybe we could extend it with `ClientBandwidth` information
    /// Has the client handle been spawned
    pub add_success: bool,
}

#[derive(Default)]
pub struct PeersState {
    pub peers: HashMap<KeyWrapper, PeerState>,
}

impl PeersState {
    pub fn get_by_x25519_key(&self, key: &x25519::PublicKey) -> Option<&PeerState> {
        let key = KeyWrapper::from(Key::new(key.to_bytes()));
        self.peers.get(&key)
    }
}

impl MockPeerControllerState {
    pub async fn register_response(
        &self,
        request: PeerControlRequestType,
        response: impl Into<RegisteredResponse>,
    ) {
        self.registered_responses
            .write()
            .await
            .entry(request)
            .or_default()
            .push_back(response.into());
    }

    pub async fn clear_registered_responses(&self) {
        self.registered_responses.write().await.clear();
    }
}

// just a helper trait to help with the duplicate code and the associated noise
trait SendDowncasted {
    fn send_downcasted(self, response: Box<dyn Any + Send + Sync>);
}

impl<T> SendDowncasted for oneshot::Sender<T>
where
    T: 'static,
{
    fn send_downcasted(self, response: Box<dyn Any + Send + Sync>) {
        if self
            .send(
                *response
                    .downcast()
                    .expect("registered response has mismatched type"),
            )
            .is_err()
        {
            panic!("attempted to send response on closed channel")
        }
    }
}

pub struct MockPeerController {
    state: MockPeerControllerState,
    request_rx: Receiver<PeerControlRequest>,
}

impl MockPeerController {
    pub fn new(state: MockPeerControllerState, request_rx: Receiver<PeerControlRequest>) -> Self {
        MockPeerController { state, request_rx }
    }

    async fn handle_request(&mut self, request: PeerControlRequest) {
        let mut res_guard = self.state.registered_responses.write().await;
        let mut peers_guard = self.state.peers.write().await;

        let typ = PeerControlRequestType::from(&request);

        let Some(registered_responses) = res_guard.get_mut(&typ) else {
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
                let key = typ.peer_key_unchecked();
                let peer = peers_guard.peers.entry(key).or_default();
                if response.success {
                    peer.add_success = true;
                }
                response_tx.send_downcasted(response.content)
            }
            PeerControlRequest::RegisterPeer { response_tx, .. } => {
                let key = typ.peer_key_unchecked();
                let peer = peers_guard.peers.entry(key).or_default();
                if response.success {
                    peer.register_success = true;
                }
                response_tx.send_downcasted(response.content)
            }
            PeerControlRequest::RemovePeer { response_tx, .. } => {
                let key = typ.peer_key_unchecked();
                if response.success {
                    peers_guard.peers.remove(&key);
                }
                response_tx.send_downcasted(response.content)
            }
            PeerControlRequest::QueryPeer { response_tx, .. } => {
                response_tx.send_downcasted(response.content)
            }
            PeerControlRequest::GetClientBandwidthByKey { response_tx, .. } => {
                response_tx.send_downcasted(response.content)
            }
            PeerControlRequest::GetClientBandwidthByIp { response_tx, .. } => {
                response_tx.send_downcasted(response.content)
            }
            PeerControlRequest::GetVerifierByKey { response_tx, .. } => {
                response_tx.send_downcasted(response.content)
            }
            PeerControlRequest::GetVerifierByIp { response_tx, .. } => {
                response_tx.send_downcasted(response.content)
            }
        }
    }

    pub async fn run(&mut self) {
        while let Some(request) = self.request_rx.recv().await {
            self.handle_request(request).await;
        }
    }
}
