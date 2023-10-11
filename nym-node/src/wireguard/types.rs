// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use nym_node_requests::api::v1::gateway::client_interfaces::wireguard::models::Nonce;
use nym_node_requests::api::v1::gateway::client_interfaces::wireguard::models::{
    Client, ClientPublicKey,
};
use std::collections::HashMap;
use std::net::SocketAddr;

pub type ClientRegistry = HashMap<SocketAddr, Client>;
pub type PendingRegistrations = HashMap<ClientPublicKey, Nonce>;
