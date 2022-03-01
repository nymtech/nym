// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::EncodedChannelPublicKey;
use cosmwasm_std::StdError;
use thiserror::Error;

/// Custom errors for contract failure conditions.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Public key of this authority is not known")]
    PublicKeyNotKnown,

    #[error("This authority has already submitted its public key - {0}")]
    PublicKeyAlreadySubmitted(EncodedChannelPublicKey),

    #[error("This sender is not an issuing authority")]
    NotAnIssuer,

    #[error("Initial share exchange is in progress")]
    InitialExchangeInProgress,
}
