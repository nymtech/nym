// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("peers in wireguard don't match with in-memory ")]
    PeerMismatch,

    #[error("{0}")]
    Defguard(#[from] defguard_wireguard_rs::error::WireguardInterfaceError),

    #[error("{0}")]
    GatewayStorageError(#[from] nym_gateway_storage::error::StorageError),
}
