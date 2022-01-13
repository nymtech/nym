// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use config::defaults::DENOM;
use cosmwasm_std::{Addr, StdError};
use mixnet_contract_common::IdentityKey;
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

    #[error("Provided active set size is bigger than the rewarded set")]
    InvalidActiveSetSize,

    #[error("Provided active set size is zero")]
    ZeroActiveSet,

    #[error("Provided rewarded set size is zero")]
    ZeroRewardedSet,

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

    #[error("Could not find any delegation information associated with mixnode {identity} for {address}")]
    NoMixnodeDelegationFound {
        identity: IdentityKey,
        address: Addr,
    },

    #[error("We tried to remove more funds then are available in the Reward pool. Wanted to remove {to_remove}, but have only {reward_pool}")]
    OutOfFunds { to_remove: u128, reward_pool: u128 },

    #[error("Received invalid rewarding interval nonce. Expected {expected}, received {received}")]
    InvalidRewardingIntervalNonce { received: u32, expected: u32 },

    #[error("Rewarding distribution is currently in progress")]
    RewardingInProgress,

    #[error("Rewarding distribution is currently not in progress")]
    RewardingNotInProgress,

    #[error("Mixnode {identity} has already been rewarded during the current rewarding interval")]
    MixnodeAlreadyRewarded { identity: IdentityKey },

    #[error("Some of mixnodes {identity} delegators are still pending reward")]
    DelegatorsPendingReward { identity: IdentityKey },

    #[error("Mixnode's {identity} operator has not been rewarded yet - cannot perform delegator rewarding until that happens")]
    MixnodeOperatorNotRewarded { identity: IdentityKey },

    #[error("Proxy address mismatch, expected {existing}, got {incoming}")]
    ProxyMismatch { existing: String, incoming: String },

    #[error("Failed to recover ed25519 public key from its base58 representation - {0}")]
    MalformedEd25519IdentityKey(String),

    #[error("Failed to recover ed25519 signature from its base58 representation - {0}")]
    MalformedEd25519Signature(String),

    #[error("Provided ed25519 signature did not verify correctly")]
    InvalidEd25519Signature,

    #[error("Profit margin percent needs to be an integer in range [0, 100], received {0}")]
    InvalidProfitMarginPercent(u8),

    #[error("Rewarded set height not set, was rewarding set determined?")]
    RewardSetHeightMapEmpty,

    #[error("Received unexpected value for the active set. Got: {received}, expected: {expected}")]
    UnexpectedActiveSetSize { received: u32, expected: u32 },

    #[error("Received unexpected value for the rewarded set. Got: {received}, expected at most: {expected}")]
    UnexpectedRewardedSetSize { received: u32, expected: u32 },

    #[error("There hasn't been sufficient delay since last rewarded set update. It was last updated at height {last_update}. The delay is {minimum_delay}. The current block height is {current_height}")]
    TooFrequentRewardedSetUpdate {
        last_update: u64,
        minimum_delay: u32,
        current_height: u64,
    },

    #[error("Can't change to the desired epoch as it's not in progress yet. It starts at {epoch_start} and finishes at {epoch_end}, while the current block time is {current_block_time}")]
    EpochNotInProgress {
        current_block_time: u64,
        epoch_start: i64,
        epoch_end: i64,
    },
}
