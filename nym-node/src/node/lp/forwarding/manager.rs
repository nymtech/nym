// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::{
    ConnectionControllerResponse, ConnectionHandlerResponse, GetConnectionHandler,
    NestedConnectionControllerRequest, NodeConnectionControllerSender,
};
use crate::node::lp::error::LpHandlerError;
use crate::node::lp::forwarding::client_connection::NestedClientConnection;
use nym_lp::peer_config::LpReceiverIndex;
use std::net::SocketAddr;
use tokio::sync::oneshot;

#[derive(Clone)]
pub struct NestedConnectionsManager {
    sender: NodeConnectionControllerSender,
}

impl NestedConnectionsManager {
    pub fn new(sender: NodeConnectionControllerSender) -> Self {
        Self { sender }
    }

    async fn send_connection_handler_request(
        &self,
        request: GetConnectionHandler,
    ) -> Result<ConnectionHandlerResponse, LpHandlerError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.sender
            .send(NestedConnectionControllerRequest::ConnectionHandler {
                request,
                response_tx,
            })
            .await
            .map_err(|_| LpHandlerError::internal("nested connection controller shut down"))?;

        response_rx.await.map_err(|_| {
            LpHandlerError::internal("nested connection controller hasn't send a response")
        })
    }

    pub(crate) async fn get_connection_handle(
        &self,
        target_gateway_lp_address: SocketAddr,
        inner_receiver_index: LpReceiverIndex,
    ) -> Result<NestedClientConnection, LpHandlerError> {
        let request = GetConnectionHandler {
            target_gateway_lp_address,
            inner_receiver_index,
        };

        let notify = match self.send_connection_handler_request(request).await?? {
            // if we have received a ready response, we can return the connection
            ConnectionControllerResponse::Ready(conn) => return Ok(conn),

            // otherwise we need to wait for the notification when it becomes available
            ConnectionControllerResponse::Pending(notify) => notify,
        };

        // TODO: timeout
        notify.notified().await;

        match self.send_connection_handler_request(request).await?? {
            // if we have received a ready response, we can return the connection
            ConnectionControllerResponse::Ready(conn) => Ok(conn),

            // otherwise we need to wait for the notification when it becomes available
            ConnectionControllerResponse::Pending(_) => Err(LpHandlerError::internal(
                "unavailable connection handler after successful notification",
            )),
        }
    }
}
