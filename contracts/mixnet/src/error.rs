// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use config::defaults::DENOM;
use cosmwasm_std::{Addr, StdError};
use mixnet_contract::IdentityKey;
use thiserror::Error;

/// Custom errors for contract failure conditions.
///
/// Add any other custom errors you like here.
/// Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Not enough funds sent for mixnode bond. (received {received}, minimum {minimum})")]
    InsufficientMixNodeBond { received: u128, minimum: u128 },

    #[error("Mixnode ({identity}) does not exist")]
    MixNodeBondNotFound { identity: IdentityKey },

    #[error("Not enough funds sent for gateway bond. (received {received}, minimum {minimum})")]
    InsufficientGatewayBond { received: u128, minimum: u128 },

    #[error("Gateway ({identity}) does not exist")]
    GatewayBondNotFound { identity: IdentityKey },

    #[error("{owner} does not seem to own any mixnodes")]
    NoAssociatedMixNodeBond { owner: Addr },

    #[error("{owner} does not seem to own any gateways")]
    NoAssociatedGatewayBond { owner: Addr },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Wrong coin denomination, you must send {}", DENOM)]
    WrongDenom,

    #[error("Received multiple coin types during staking")]
    MultipleDenoms,

    #[error("No coin was sent for the bonding, you must send {}", DENOM)]
    NoBondFound,

    #[error("The bond reward rate for mixnode was set to be lower than 1")]
    DecreasingMixnodeBondReward,

    #[error("The delegation reward rate for mixnode was set to be lower than 1")]
    DecreasingMixnodeDelegationReward,

    #[error("The node had uptime larger than 100%")]
    UnexpectedUptime,

    #[error("This address has already bonded a mixnode")]
    AlreadyOwnsMixnode,

    #[error("This address has already bonded a gateway")]
    AlreadyOwnsGateway,

    #[error("Mixnode with this identity already exists. Its owner is {owner}")]
    DuplicateMixnode { owner: Addr },

    #[error("Gateway with this identity already exists. Its owner is {owner}")]
    DuplicateGateway { owner: Addr },

    #[error("No funds were provided for the delegation")]
    EmptyDelegation,

    #[error("Request did not come from the node owner ({owner})")]
    InvalidSender { owner: Addr },

    #[error("Could not find any delegation information associated with mixnode {identity} for {address}")]
    NoMixnodeDelegationFound {
        identity: IdentityKey,
        address: Addr,
    },
    #[error("Overflow error!")]
    Overflow(#[from] cosmwasm_std::OverflowError),

    #[error("Invalid ratio")]
    Ratio(#[from] mixnet_contract::error::MixnetContractError)
}
