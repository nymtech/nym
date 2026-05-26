// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to serialize json data: {source}")]
    JsonSerializationFailure {
        #[from]
        source: serde_json::Error,
    },

    #[error(transparent)]
    WireguardError {
        #[from]
        source: nym_wireguard_types::Error,
    },

    #[cfg(feature = "client")]
    #[error("node {node_id} has provided malformed host information ({host}: {source})")]
    MalformedHost {
        host: String,

        node_id: u32,

        #[source]
        source: Box<crate::api::client::NymNodeApiClientError>,
    },

    #[cfg(feature = "client")]
    #[error("failed to query node {node_id} at host {host}: {source}")]
    QueryFailure {
        host: String,

        node_id: u32,

        #[source]
        source: Box<crate::api::client::NymNodeApiClientError>,
    },

    #[cfg(feature = "client")]
    #[error(
        "node {node_id} with host '{host}' doesn't seem to expose its declared http port nor any of the standard API ports, i.e.: 80, 443 or {}",
        nym_network_defaults::DEFAULT_NYM_NODE_HTTP_PORT
    )]
    NoHttpPortsAvailable { host: String, node_id: u32 },

    #[cfg(feature = "client")]
    #[error("could not verify signed host information for node {node_id}")]
    MissignedHostInformation { node_id: u32 },

    #[cfg(feature = "client")]
    #[error("identity of node {node_id} does not match. expected {expected} but got {got}")]
    MismatchedIdentity {
        node_id: u32,
        expected: String,
        got: String,
    },
}
