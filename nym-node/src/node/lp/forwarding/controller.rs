// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::control::egress::connection::NestedNodeControlSender;
use crate::node::lp::forwarding::{
    ConnectionHandlerResponse, GetConnectionHandler, NestedConnectionControllerRequest,
};
use nym_crypto::asymmetric::ed25519;
use std::collections::HashMap;

pub const CONTROL_CHANNEL_SIZE: usize = 64;

/// Keep track of connections to the exit gateway
pub(crate) struct NestedConnectionsController {
    /// Handle channel for sending requests to this controller
    sender: super::NodeConnectionControllerSender,

    /// Channel for receiving requests in this controller
    receiver: super::NodeConnectionControllerReceiver,

    /// Handles to the active nested node connections
    nodes_handles: HashMap<ed25519::PublicKey, NestedNodeControlSender>,
}

impl NestedConnectionsController {
    pub(crate) fn request_sender(&self) -> super::NodeConnectionControllerSender {
        self.sender.clone()
    }

    async fn handle_get_connection_handler(
        &self,
        request: GetConnectionHandler,
    ) -> ConnectionHandlerResponse {
        todo!()
    }

    async fn handle_request(&self, request: NestedConnectionControllerRequest) {
        match request {
            NestedConnectionControllerRequest::ConnectionHandler {
                request,
                response_tx,
            } => {
                response_tx
                    .send(self.handle_get_connection_handler(request).await)
                    .ok();
            }
        }
    }
}
