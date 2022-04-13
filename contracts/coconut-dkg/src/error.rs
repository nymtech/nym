// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::Blacklisting;
use cosmwasm_std::StdError;
use thiserror::Error;

use config::defaults::DENOM;

/// Custom errors for contract failure conditions.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Received multiple coin types")]
    MultipleDenoms,

    #[error("Wrong coin denomination, you must send {}", DENOM)]
    WrongDenom,

    #[error("This dealer has been blacklisted - {reason}")]
    BlacklistedDealer { reason: Blacklisting },

    #[error("This potential dealer is not a validator")]
    NotAValidator,
}
