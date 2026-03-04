// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::error::LpHandlerError;
use crate::node::lp::forwarding::client_connection::NestedClientConnection;
use nym_crypto::asymmetric::ed25519;
use nym_lp::peer_config::LpReceiverIndex;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{Notify, oneshot};

pub(crate) mod client_connection;
pub(crate) mod controller;
pub(crate) mod manager;

pub(crate) type NodeConnectionControllerReceiver = Receiver<GetConnectionHandler>;
pub(crate) type NodeConnectionControllerSender = Sender<GetConnectionHandler>;

pub(crate) enum ConnectionControllerResponse<T> {
    /// The response is immediately available
    Ready(T),

    /// The response is in the process of being resolved. It will be ready once the returned
    /// notify resolves. At this point the caller should repeat the query
    Pending(Arc<Notify>),
}

pub(crate) type ControllerResponse<T> = Result<ConnectionControllerResponse<T>, LpHandlerError>;

pub(crate) type ConnectionHandlerResponse = ControllerResponse<NestedClientConnection>;

pub(crate) enum NestedConnectionControllerRequest {
    /// Attempt to retrieve or create a handle to an exit gateway connection.
    /// If the connection doesn't exist, it will be established
    ConnectionHandler {
        request: GetConnectionHandler,
        response_tx: oneshot::Sender<ConnectionHandlerResponse>,
    },
}

pub(crate) struct GetConnectionHandler {
    /// Target gateway's Ed25519 identity
    pub target_gateway: ed25519::PublicKey,

    /// Target gateway's LP address
    pub target_gateway_lp_address: SocketAddr,

    /// Receiver index on the inner packet
    pub inner_receiver_index: LpReceiverIndex,
}
