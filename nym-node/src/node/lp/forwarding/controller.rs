// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::control::egress::connection::NestedNodeControlSender;
use crate::node::lp::directory::LpNodes;
use crate::node::lp::error::LpHandlerError;
use crate::node::lp::forwarding::{
    ConnectionControllerResponse, ConnectionHandlerResponse, ControllerResponse,
    GetConnectionHandler, NestedConnectionControllerRequest,
};
use nym_topology::NodeId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::info;

pub const CONTROL_CHANNEL_SIZE: usize = 64;

pub(crate) enum NodeHandle {
    Established(NestedNodeControlSender),
    Pending(Arc<Notify>),
}

/// Keep track of connections to the exit gateway
pub struct NestedConnectionsController {
    /// Handle channel for sending requests to this controller
    sender: super::NodeConnectionControllerSender,

    /// Channel for receiving requests in this controller
    receiver: super::NodeConnectionControllerReceiver,

    /// Map of all LP node ip addresses to their details (and ids)
    lp_nodes: LpNodes,

    /// Handles to the active nested node connections
    nodes_handles: HashMap<NodeId, NodeHandle>,

    /// Shutdown token
    shutdown: nym_task::ShutdownToken,
}

impl NestedConnectionsController {
    pub fn new(lp_nodes: LpNodes, shutdown: nym_task::ShutdownToken) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(CONTROL_CHANNEL_SIZE);

        Self {
            sender,
            receiver,
            lp_nodes,
            nodes_handles: HashMap::new(),
            shutdown,
        }
    }

    pub fn request_sender(&self) -> super::NodeConnectionControllerSender {
        self.sender.clone()
    }

    async fn handle_get_connection_handler(
        &mut self,
        request: GetConnectionHandler,
    ) -> ConnectionHandlerResponse {
        let ip = request.target_gateway_lp_address.ip();

        let Some(node_id) = self.lp_nodes.get_node_id(ip) else {
            return Err(LpHandlerError::NotLpNode { ip_addr: ip });
        };

        match self.nodes_handles.get(&node_id) {
            Some(NodeHandle::Established(handle)) => {
                todo!()
            }
            Some(NodeHandle::Pending(notify)) => {
                Ok(ConnectionControllerResponse::Pending(notify.clone()))
            }
            None => {
                let (res, notify) = ConnectionControllerResponse::new_pending();
                self.nodes_handles
                    .insert(node_id, NodeHandle::Pending(notify.clone()));

                // create a new connection and return a pending response
                todo!();
                return Ok(res);
            }
        }
    }

    async fn handle_request(&mut self, request: NestedConnectionControllerRequest) {
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

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    break;
                }
                Some(request) = self.receiver.recv() => {
                    self.handle_request(request).await;
                }
            }
        }

        info!("Nested connection controller shutdown complete");
    }
}
