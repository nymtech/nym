// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Coin;
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum NymPoolContractError {
    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error(transparent)]
    StdErr(#[from] cosmwasm_std::StdError),

    #[error("{addr} is not a permitted granter")]
    InvalidGranter { addr: String },

    #[error("invalid coin denomination. got {got}, but expected {expected}")]
    InvalidDenom { expected: String, got: String },

    #[error("there already exists an active grant for {grantee}. it was granted by {granter} at block height {created_at_height}")]
    GrantAlreadyExist {
        granter: String,
        grantee: String,
        created_at_height: u64,
    },

    #[error("could not find any active grants for {grantee}")]
    GrantNotFound { grantee: String },

    #[error("the provided timestamp value ({timestamp}) is set in the past. the current block timestamp is {current_block_timestamp}")]
    TimestampInThePast {
        timestamp: u64,
        current_block_timestamp: u64,
    },

    #[error("there are not enough tokens to process this grant. {available} are available, but {requested_grant} was requested.")]
    InsufficientTokens {
        available: Coin,
        requested_grant: Coin,
    },

    #[error("the period length can't be zero")]
    ZeroAllowancePeriod,

    #[error("the periodic spend limit of {periodic} was set to be higher than the total spend limit {total_limit}")]
    PeriodicGrantOverSpendLimit { periodic: Coin, total_limit: Coin },

    #[error("the accumulation spend limit of {accumulation} was set to be lower than the periodic grant amount of {periodic_grant}")]
    AccumulationBelowGrantAmount {
        accumulation: Coin,
        periodic_grant: Coin,
    },

    #[error("the accumulation spend limit of {accumulation} was set to be higher than the total spend limit of {total_limit}")]
    AccumulationOverSpendLimit {
        accumulation: Coin,
        total_limit: Coin,
    },

    #[error("the specified delayed allowance would never be available. it would become active at {available_timestamp} yet it expires at {expiration_timestamp}")]
    UnattainableDelayedAllowance {
        expiration_timestamp: u64,
        available_timestamp: u64,
    },
}
