// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::error::LpHandlerError;
use crate::node::lp::forwarding::client_connection::NestedClientConnection;
use nym_lp::peer_config::LpReceiverIndex;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{Notify, oneshot};

pub mod client_connection;
pub mod controller;
pub mod manager;

pub type NodeConnectionControllerReceiver = Receiver<NestedConnectionControllerRequest>;
pub type NodeConnectionControllerSender = Sender<NestedConnectionControllerRequest>;

pub(crate) enum ConnectionControllerResponse<T> {
    /// The response is immediately available
    Ready(T),

    /// The response is in the process of being resolved. It will be ready once the returned
    /// notify resolves. At this point the caller should repeat the query
    Pending(Arc<Notify>),
}

impl<T> ConnectionControllerResponse<T> {
    pub fn new_pending() -> (Self, Arc<Notify>) {
        let notify = Arc::new(Notify::new());
        (
            ConnectionControllerResponse::Pending(notify.clone()),
            notify,
        )
    }
}

pub type ControllerResponse<T> = Result<ConnectionControllerResponse<T>, LpHandlerError>;

pub type ConnectionHandlerResponse = ControllerResponse<NestedClientConnection>;

pub enum NestedConnectionControllerRequest {
    /// Attempt to retrieve or create a handle to an exit gateway connection.
    /// If the connection doesn't exist, it will be established
    ConnectionHandler {
        request: GetConnectionHandler,
        response_tx: oneshot::Sender<ConnectionHandlerResponse>,
    },
}

#[derive(Copy, Clone)]
pub(crate) struct GetConnectionHandler {
    /// Target gateway's LP address
    pub target_gateway_lp_address: SocketAddr,

    /// Receiver index on the inner packet
    pub inner_receiver_index: LpReceiverIndex,
}
