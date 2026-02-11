// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::NodeConnectionControllerSender;
use crate::node::lp::error::LpHandlerError;
use crate::node::lp::forwarding::client_connection::NestedClientConnection;
use nym_crypto::asymmetric::ed25519;
use std::net::SocketAddr;

pub(crate) struct NestedConnectionsManager {
    sender: NodeConnectionControllerSender,
}

impl NestedConnectionsManager {
    pub(crate) async fn get_connection_handle(
        &self,
        target_gateway: ed25519::PublicKey,
        target_gateway_lp_address: SocketAddr,
    ) -> Result<NestedClientConnection, LpHandlerError> {
        // let request = GetConnectionHandler {
        //     target_gateway,
        //     target_gateway_lp_address,
        // };

        todo!()
    }
}
