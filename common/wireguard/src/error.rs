// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Defguard(#[from] defguard_wireguard_rs::error::WireguardInterfaceError),

    #[error("internal {0}")]
    Internal(String),

    #[error("storage should have the requested bandwidth entry")]
    MissingClientBandwidthEntry,

    #[error("kernel should have the requested client entry: {0}")]
    MissingClientKernelEntry(String),

    #[error("{0}")]
    GatewayStorage(#[from] nym_gateway_storage::error::GatewayStorageError),

    #[error("{0}")]
    SystemTime(#[from] std::time::SystemTimeError),
}

pub type Result<T> = std::result::Result<T, Error>;
