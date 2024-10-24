// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("traffic byte data needs to be increasing")]
    InconsistentConsumedBytes,

    #[error("{0}")]
    Defguard(#[from] defguard_wireguard_rs::error::WireguardInterfaceError),

    #[error("internal {0}")]
    Internal(String),

    #[error("storage should have the requested bandwidht entry")]
    MissingClientBandwidthEntry,

    #[error("{0}")]
    GatewayStorage(#[from] nym_gateway_storage::error::StorageError),

    #[error("{0}")]
    SystemTime(#[from] std::time::SystemTimeError),
}
