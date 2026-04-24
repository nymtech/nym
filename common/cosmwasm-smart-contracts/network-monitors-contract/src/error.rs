// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum NetworkMonitorsContractError {
    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error("unauthorised")]
    Unauthorized,

    #[error("address {addr} is not an authorised orchestrator")]
    NotAnOrchestrator { addr: Addr },

    #[error("Failed to recover x25519 public key from its base58 representation: {0}")]
    MalformedX25519AgentNoiseKey(String),

    #[error(transparent)]
    StdErr(#[from] cosmwasm_std::StdError),
}
