// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nym_node::Role;
use crate::{
    EpochEventId, EpochState, IntervalEventId, NodeId, OperatingCostRange, ProfitMarginRange,
};
use contracts_common::signing::verifier::ApiVerifierError;
use contracts_common::Percent;
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum MixnetContractError {
    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error("{source}")]
    StdErr {
        #[from]
        source: cosmwasm_std::StdError,
    },

    #[error("Attempted to subtract decimals with overflow ({minuend}.sub({subtrahend}))")]
    OverflowDecimalSubtraction {
        minuend: Decimal,
        subtrahend: Decimal,
    },

    #[error("Attempted to subtract with overflow ({minuend}.sub({subtrahend}))")]
    OverflowSubtraction { minuend: u64, subtrahend: u64 },

    #[error("Not enough funds sent for node pledge. (received {received}, minimum {minimum})")]
    InsufficientPledge { received: Coin, minimum: Coin },

    #[error(
        "the provided value for node host is too long. it must not be longer than 255 characters"
    )]
    HostTooLong,

    #[error(
        "the provided node identity public key is not a correctly encoded base58 slice of 32 bytes"
    )]
    InvalidPubKey,

    #[error("Attempted to reduce node pledge ({current}{denom} - {decrease_by}{denom}) below the minimum amount: {minimum}{denom}")]
    InvalidPledgeReduction {
        current: Uint128,
        decrease_by: Uint128,
        minimum: Uint128,
        denom: String,
    },

    #[error("A pledge change is already pending in this epoch. The event id: {pending_event_id}")]
    PendingPledgeChange { pending_event_id: EpochEventId },

    #[error(
        "A cost params change is already pending in this epoch. The event id: {pending_event_id}"
    )]
    PendingParamsChange { pending_event_id: IntervalEventId },

    #[error("Not enough funds sent for node delegation. (received {received}, minimum {minimum})")]
    InsufficientDelegation { received: Coin, minimum: Coin },

    #[error("Node ({node_id}) does not exist")]
    NymNodeBondNotFound { node_id: NodeId },

    #[error("Mixnode ({mix_id}) does not exist")]
    MixNodeBondNotFound { mix_id: NodeId },

    #[error("{owner} does not seem to own any mixnodes")]
    NoAssociatedMixNodeBond { owner: Addr },

    #[error("{owner} does not seem to own any gateways")]
    NoAssociatedGatewayBond { owner: Addr },

    #[error("{owner} does not seem to own any nodes")]
    NoAssociatedNodeBond { owner: Addr },

    #[error("This address has already bonded a mixnode")]
    AlreadyOwnsMixnode,

    #[error("This address has already bonded a gateway")]
    AlreadyOwnsGateway,

    #[error("This address has already bonded a nym-node")]
    AlreadyOwnsNymNode,

    #[error("Gateway with this identity already exists. Its owner is {owner}")]
    DuplicateGateway { owner: Addr },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("No tokens were sent for the bonding")]
    NoBondFound,

    #[error("No funds were provided for the delegation")]
    EmptyDelegation,

    #[error("Wrong coin denomination. Received: {received}, expected: {expected}")]
    WrongDenom { received: String, expected: String },

    #[error("Received multiple coin types during staking")]
    MultipleDenoms,

    #[error("Proxy address ({received}) is not set to the vesting contract ({vesting_contract})")]
    ProxyIsNotVestingContract {
        received: Addr,
        vesting_contract: Addr,
    },

    #[error("Failed to recover ed25519 public key from its base58 representation - {0}")]
    MalformedEd25519IdentityKey(String),

    #[error("Failed to recover ed25519 signature from its base58 representation - {0}")]
    MalformedEd25519Signature(String),

    #[error("Provided ed25519 signature did not verify correctly")]
    InvalidEd25519Signature,

    #[error("Can't perform the specified action as the current epoch is still progress. It started at {epoch_start} and finishes at {epoch_end}, while the current block time is {current_block_time}")]
    EpochInProgress {
        current_block_time: u64,
        epoch_start: i64,
        epoch_end: i64,
    },

    #[error("attempted to reward a gateway node - this has not been fully integrated yet")]
    GatewayRewarding,

    #[error("node {node_id} has already been rewarded during the current rewarding epoch ({absolute_epoch_id})")]
    NodeAlreadyRewarded {
        node_id: NodeId,
        absolute_epoch_id: u32,
    },

    #[error("node {node_id} hasn't been assigned the role of {role} for this epoch")]
    IncorrectEpochRole { node_id: NodeId, role: Role },

    #[error("Mixnode {mix_id} hasn't been selected to the rewarding set in this epoch ({absolute_epoch_id})")]
    MixnodeNotInRewardedSet {
        mix_id: NodeId,
        absolute_epoch_id: u32,
    },

    #[error("Mixnode {mix_id} is currently in the process of unbonding")]
    MixnodeIsUnbonding { mix_id: NodeId },

    #[error("Node {node_id} is currently in the process of unbonding")]
    NodeIsUnbonding { node_id: NodeId },

    #[error("Mixnode {mix_id} has already unbonded")]
    MixnodeHasUnbonded { mix_id: NodeId },

    #[error("The contract has ended up in a state that was deemed impossible: {comment}")]
    InconsistentState { comment: String },

    #[error(
        "Could not find any delegation information associated with node {node_id} for {address} (proxy: {proxy:?})"
    )]
    NodeDelegationNotFound {
        node_id: NodeId,
        address: String,
        proxy: Option<String>,
    },

    #[error("Provided message to update rewarding params did not contain any updates")]
    EmptyParamsChangeMsg,

    #[error("one of the roles in the new active set is empty")]
    EmptyRoleAssignment,

    #[error("the number of mixnodes in the rewarded set is not divisible by the number of mix-layers (3)")]
    UnevenLayerAssignment,

    #[error("provided active set is bigger than the rewarded set")]
    InvalidActiveSetSize,

    #[error("Invalid layer expected 1, 2 or 3, got {0}")]
    InvalidLayer(u8),

    #[error("Feature is not yet implemented")]
    NotImplemented,

    #[error("epochs_in_interval must be > 0")]
    EpochsInIntervalZero,

    #[error("epoch duration must be > 0")]
    EpochDurationZero,

    #[error("attempted to perform the operation with 0 coins. This is not allowed")]
    ZeroCoinAmount,

    #[error("this validator ({current_validator}) is not the one responsible for advancing this epoch. It's responsibility of {chosen_validator}.")]
    RewardingValidatorMismatch {
        current_validator: Addr,
        chosen_validator: Addr,
    },

    #[error("the epoch is currently in the process of being advanced. (the state is {current_state}) Please try sending your transaction again once this has finished")]
    EpochAdvancementInProgress { current_state: EpochState },

    #[error("the epoch is in an unexpected state. expected 'mix rewarding' state, but we're in {current_state} instead.")]
    UnexpectedNonRewardingEpochState { current_state: EpochState },

    #[error("attempted to reward mixnode out of order. Attempted to reward {attempted_to_reward} while last rewarded was {last_rewarded}.")]
    RewardingOutOfOrder {
        last_rewarded: NodeId,
        attempted_to_reward: NodeId,
    },

    #[error("the epoch is currently not in the 'event reconciliation' state. (the state is {current_state})")]
    EpochNotInEventReconciliationState { current_state: EpochState },

    #[error(
        "the epoch is currently not in the 'role assignment' state. (the state is {current_state})"
    )]
    EpochNotInRoleAssignmentState { current_state: EpochState },

    #[error("unexpected role assignment. got: {got} while expected: {expected}")]
    UnexpectedRoleAssignment { expected: Role, got: Role },

    #[error("attempted to assign an invalid number of nodes for a role of {role}. got {assigned}, but the maximum allowed is {allowed}")]
    IllegalRoleCount {
        role: Role,
        assigned: u32,
        allowed: u32,
    },

    #[error("the epoch is currently not in the 'epoch advancement' state. (the state is {current_state})")]
    EpochNotInAdvancementState { current_state: EpochState },

    #[error("failed to verify message signature: {source}")]
    SignatureVerificationFailure {
        #[from]
        source: ApiVerifierError,
    },

    #[error("this operation is no longer allowed to be performed with vesting tokens. please move them to your liquid balance and try again")]
    DisabledVestingOperation,

    #[error(
        "this mixnode has not been bonded with the vesting tokens or has already been migrated"
    )]
    NotAVestingMixnode,

    #[error("this delegation has not been performed with the vesting tokens or has already been migrated")]
    NotAVestingDelegation,

    #[error("the provided profit margin ({provided}) is outside the allowed range: {range}")]
    ProfitMarginOutsideRange {
        provided: Percent,
        range: ProfitMarginRange,
    },

    #[error("the provided interval operating cost ({provided}{denom}) is outside the allowed range: {range}")]
    OperatingCostOutsideRange {
        denom: String,
        provided: Uint128,
        range: OperatingCostRange,
    },

    #[error(
        "currently it's not possible to migrate nodes bonded with vesting tokens into a nym-node. please perform vesting->liquid migration first."
    )]
    VestingNodeMigration,

    #[error("value {got} does not correspond to any known node role")]
    UnknownRoleRepresentation { got: u8 },

    #[error("the total work for this epoch seems to be bigger than 1.0!")]
    TotalWorkAboveOne,
}

impl MixnetContractError {
    pub fn inconsistent_state<S: Into<String>>(comment: S) -> Self {
        MixnetContractError::InconsistentState {
            comment: comment.into(),
        }
    }
}
