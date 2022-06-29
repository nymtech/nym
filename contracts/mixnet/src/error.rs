// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use config::defaults::MIX_DENOM;
use cosmwasm_std::{Addr, StdError};
use mixnet_contract_common::{error::MixnetContractError, IdentityKey};
use thiserror::Error;

/// Custom errors for contract failure conditions.
///
/// Add any other custom errors you like here.
/// Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
#[deprecated("use the one defined in common")]
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("MIXNET ({}): {0}", line!())]
    Std(#[from] StdError),
    // #[error("MIXNET ({}): Not enough funds sent for mixnode bond. (received {received}, minimum {minimum})", line!())]
    // InsufficientMixNodeBond { received: u128, minimum: u128 },
    //
    // #[error("MIXNET ({}): Mixnode ({identity}) does not exist", line!())]
    // MixNodeBondNotFound { identity: IdentityKey },
    //
    // #[error("MIXNET ({}): Not enough funds sent for gateway bond. (received {received}, minimum {minimum})", line!())]
    // InsufficientGatewayBond { received: u128, minimum: u128 },
    //
    // #[error("MIXNET ({}): {owner} does not seem to own any mixnodes", line!())]
    // NoAssociatedMixNodeBond { owner: Addr },
    //
    // #[error("MIXNET ({}): {owner} does not seem to own any gateways", line!())]
    // NoAssociatedGatewayBond { owner: Addr },
    //
    // #[error("MIXNET ({}): Unauthorized", line!())]
    // Unauthorized,
    //
    // #[error("MIXNET ({}): Wrong coin denomination, you must send {}", line!(), MIX_DENOM.base)]
    // WrongDenom,
    //
    // #[error("MIXNET ({}): Received multiple coin types during staking", line!())]
    // MultipleDenoms,
    //
    // #[error("MIXNET ({}): No coin was sent for the bonding, you must send {}", line!(), MIX_DENOM.base)]
    // NoBondFound,
    //
    // #[error("MIXNET ({}): Provided active set size is bigger than the rewarded set", line!())]
    // InvalidActiveSetSize,
    //
    // #[error("MIXNET ({}): Provided active set size is zero", line!())]
    // ZeroActiveSet,
    //
    // #[error("MIXNET ({}): Provided rewarded set size is zero", line!())]
    // ZeroRewardedSet,
    //
    // #[error("MIXNET ({}): This address has already bonded a mixnode", line!())]
    // AlreadyOwnsMixnode,
    //
    // #[error("MIXNET ({}): This address has already bonded a gateway", line!())]
    // AlreadyOwnsGateway,
    //
    // #[error("MIXNET ({}): Mixnode with this identity already exists. Its owner is {owner}", line!())]
    // DuplicateMixnode { owner: Addr },
    //
    // #[error("MIXNET ({}): Gateway with this identity already exists. Its owner is {owner}", line!())]
    // DuplicateGateway { owner: Addr },
    //
    // #[error("MIXNET ({}): No funds were provided for the delegation", line!())]
    // EmptyDelegation,
    //
    // #[error("MIXNET ({}): Could not find any delegation information associated with mixnode {identity} for {address}", line!())]
    // NoMixnodeDelegationFound {
    //     identity: IdentityKey,
    //     address: String,
    // },
    //
    // #[error("MIXNET ({}): We tried to remove more funds then are available in the Reward pool. Wanted to remove {to_remove}, but have only {reward_pool}", line!())]
    // OutOfFunds { to_remove: u128, reward_pool: u128 },
    //
    // #[error("MIXNET ({}): Received invalid interval id. Expected {expected}, received {received}", line!())]
    // InvalidIntervalId { received: u32, expected: u32 },
    //
    // #[error("MIXNET ({}): Mixnode {identity} has already been rewarded during the current rewarding interval", line!())]
    // MixnodeAlreadyRewarded { identity: IdentityKey },
    //
    // #[error("MIXNET ({}): Some of mixnodes {identity} delegators are still pending reward", line!())]
    // DelegatorsPendingReward { identity: IdentityKey },
    //
    // #[error("MIXNET ({}): Mixnode's {identity} operator has not been rewarded yet - cannot perform delegator rewarding until that happens", line!())]
    // MixnodeOperatorNotRewarded { identity: IdentityKey },
    //
    // #[error("MIXNET ({}): Proxy address mismatch, expected {existing}, got {incoming}", line!())]
    // ProxyMismatch { existing: String, incoming: String },
    //
    // #[error("MIXNET ({}): Failed to recover ed25519 public key from its base58 representation - {0}", line!())]
    // MalformedEd25519IdentityKey(String),
    //
    // #[error("MIXNET ({}): Failed to recover ed25519 signature from its base58 representation - {0}", line!())]
    // MalformedEd25519Signature(String),
    //
    // #[error("MIXNET ({}): Provided ed25519 signature did not verify correctly", line!())]
    // InvalidEd25519Signature,
    //
    // #[error("MIXNET ({}): Profit margin percent needs to be an integer in range [0, 100], received {0}", line!())]
    // InvalidProfitMarginPercent(u8),
    //
    // #[error("MIXNET ({}): Rewarded set height not set, was rewarding set determined?", line!())]
    // RewardSetHeightMapEmpty,
    //
    // #[error("MIXNET ({}): Received unexpected value for the active set. Got: {received}, expected: {expected}", line!())]
    // UnexpectedActiveSetSize { received: u32, expected: u32 },
    //
    // #[error("MIXNET ({}): Received unexpected value for the rewarded set. Got: {received}, expected at most: {expected}", line!())]
    // UnexpectedRewardedSetSize { received: u32, expected: u32 },
    //
    // #[error("MIXNET ({}): There hasn't been sufficient delay since last rewarded set update. It was last updated at height {last_update}. The delay is {minimum_delay}. The current block height is {current_height}", line!())]
    // TooFrequentRewardedSetUpdate {
    //     last_update: u64,
    //     minimum_delay: u64,
    //     current_height: u64,
    // },
    //
    // #[error("MIXNET ({}): Can't change to the desired interval as it's not in progress yet. It starts at {interval_start} and finishes at {interval_end}, while the current block time is {current_block_time}", line!())]
    // IntervalNotInProgress {
    //     current_block_time: u64,
    //     interval_start: i64,
    //     interval_end: i64,
    // },
    //
    // #[error("MIXNET ({}): Can't change to the desired interval as it's not in progress yet. It starts at {epoch_start} and finishes at {epoch_end}, while the current block time is {current_block_time}", line!())]
    // EpochNotInProgress {
    //     current_block_time: u64,
    //     epoch_start: i64,
    //     epoch_end: i64,
    // },
    //
    // #[error("MIXNET ({}): Can't change to the desired interval as it hasn't started yet. It starts at {epoch_start} and finishes at {epoch_end}, while the current block time is {current_block_time}", line!())]
    // EpochNotStarted {
    //     current_block_time: u64,
    //     epoch_start: i64,
    //     epoch_end: i64,
    // },
    //
    // #[error("MIXNET ({}): Can't change to the desired interval as it's in progress. It starts at {epoch_start} and finishes at {epoch_end}, while the current block time is {current_block_time}", line!())]
    // EpochInProgress {
    //     current_block_time: u64,
    //     epoch_start: i64,
    //     epoch_end: i64,
    // },
    //
    // #[error("Could not cast reward to a u128, this should be impossible, at {}", line!())]
    // CastError,
    // #[error("{source}")]
    // MixnetCommonError {
    //     #[from]
    //     source: MixnetContractError,
    // },
    // #[error("No rewards to claim for mixnode {identity} for {address}")]
    // NoRewardsToClaim { identity: String, address: String },
    //
    // #[error("Epoch not initialized yet!")]
    // EpochNotInitialized,
    //
    // #[error("Invalid address: {0}")]
    // InvalidAddress(String),
    //
    // #[error("Pending {kind} event  already exists at block {block_height} for mixnode {identity}")]
    // DelegationEventAlreadyPending {
    //     block_height: u64,
    //     identity: String,
    //     kind: String, // delegation | undelegation
    // },
    // #[error("Attempted to subsctract more then the total delegation, this MUST never happen! mix: {mix_identity}, total_node_delegation {total_node_delegation}, to_subtract {to_subtract}")]
    // TotalDelegationSubOverflow {
    //     mix_identity: String,
    //     total_node_delegation: u128,
    //     to_subtract: u128,
    // },
    // #[error("Profit margin can be updated only once during a rolling 30 day interval, last update was at {last_update_time} and current block time is {current_block_time}")]
    // UpdatePMTooSoon {
    //     last_update_time: u64,
    //     current_block_time: u64,
    // },
}
