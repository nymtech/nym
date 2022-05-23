// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

pub use crate::dealer::{
    BlacklistedDealer, Blacklisting, BlacklistingReason, BlacklistingResponse, DealerDetails,
    PagedBlacklistingResponse, PagedDealerResponse,
};
pub use contracts_common::commitment::ContractSafeCommitment;
pub use cosmwasm_std::{Addr, Coin};

pub type BlockHeight = u64;
pub type EncodedEd25519PublicKey = String;
pub type EncodedEd25519PublicKeyRef<'a> = &'a str;
pub type EncodedBTEPublicKeyWithProof = String;
pub type EncodedBTEPublicKeyWithProofRef<'a> = &'a str;
pub type NodeIndex = u64;
pub type Threshold = u64;
pub type EpochId = u32;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct Epoch {
    pub id: EpochId,
    pub state: EpochState,
    pub state_duration: EpochStateDuration,

    // TODO: need to ponder a bit whether it's actually a property of a particular epoch
    pub system_threshold: Threshold,
}

impl Ord for Epoch {
    // we don't care about `system_threshold` when ordering
    fn cmp(&self, other: &Self) -> Ordering {
        if self.id != other.id {
            self.id.cmp(&other.id)
        } else {
            self.state.cmp(&other.state)
        }
    }
}

impl PartialOrd for Epoch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Epoch {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Epoch {} at state {}", self.id, self.state)
    }
}

impl Epoch {
    pub fn next_state(
        &self,
        current_time: Option<BlockHeight>,
        end_time: Option<BlockHeight>,
    ) -> Option<Self> {
        let mut advance_epoch = false;
        let state = match self.state.next() {
            Some(next_state) => next_state,
            None => {
                advance_epoch = true;
                EpochState::PublicKeySubmission
            }
        };

        let id = if advance_epoch { self.id + 1 } else { self.id };

        let new_state_start = current_time.unwrap_or(self.state_duration.finish_by?);

        Some(Epoch {
            id,
            state,
            state_duration: EpochStateDuration {
                begun_at: new_state_start,
                finish_by: end_time,
            },
            system_threshold: self.system_threshold,
        })
    }
}

// currently (it is still extremely likely to change, we might be able to get rid of verification key-related complaints),
// the epoch can be in the following states (in order):
// 1. PublicKeySubmission -> potential dealers are submitting their BTE and ed25519 public keys to participate in dealing exchange
// 2. DealingExchange -> the actual (off-chain) dealing exchange is happening
// 3. ComplaintSubmission -> receivers submitting evidence of other dealers sending malformed data
// 4. ComplaintVoting -> (if any complaints were submitted) receivers voting on the validity of the evidence provided
// 5. VerificationKeySubmission -> receivers submitting their partial (and master) verification keys
// 6. VerificationKeyMismatchSubmission -> receivers / watchers raising issue that the submitted VK are mismatched with their local derivations
// 7. VerificationKeyMismatchVoting -> (if any complaints were submitted) receivers voting on received mismatches
// 8. InProgress -> all receivers have all their secrets derived and all is good
//
// Note: It's important that the variant ordering is not changed otherwise it would mess up the derived `PartialOrd`
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum EpochState {
    PublicKeySubmission,
    DealingExchange,
    ComplaintSubmission,
    ComplaintVoting,
    VerificationKeySubmission,
    VerificationKeyMismatchSubmission,
    VerificationKeyMismatchVoting,
    InProgress,
}

impl Display for EpochState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EpochState::PublicKeySubmission => write!(f, "PublicKeySubmission"),
            EpochState::DealingExchange => write!(f, "DealingExchange"),
            EpochState::ComplaintSubmission => write!(f, "ComplaintSubmission"),
            EpochState::ComplaintVoting => write!(f, "ComplaintVoting"),
            EpochState::VerificationKeySubmission => write!(f, "VerificationKeySubmission"),
            EpochState::VerificationKeyMismatchSubmission => {
                write!(f, "VerificationKeyMismatchSubmission")
            }
            EpochState::VerificationKeyMismatchVoting => {
                write!(f, "VerificationKeyMismatchVoting")
            }
            EpochState::InProgress => write!(f, "InProgress"),
        }
    }
}

impl EpochState {
    pub fn next(self) -> Option<Self> {
        match self {
            EpochState::PublicKeySubmission => Some(EpochState::DealingExchange),
            EpochState::DealingExchange => Some(EpochState::ComplaintSubmission),
            EpochState::ComplaintSubmission => Some(EpochState::ComplaintVoting),
            EpochState::ComplaintVoting => Some(EpochState::VerificationKeySubmission),
            EpochState::VerificationKeySubmission => {
                Some(EpochState::VerificationKeyMismatchSubmission)
            }
            EpochState::VerificationKeyMismatchSubmission => {
                Some(EpochState::VerificationKeyMismatchVoting)
            }
            EpochState::VerificationKeyMismatchVoting => Some(EpochState::InProgress),
            EpochState::InProgress => None,
        }
    }

    pub fn all_until(&self, end: Self) -> Vec<Self> {
        let mut states = vec![*self];
        while states.last().unwrap() != &end {
            let next_state = states.last().unwrap().next().expect("somehow reached the end of state diff -> this should be impossible under any circumstances!");
            states.push(next_state);
        }

        states
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct EpochStateDuration {
    pub begun_at: BlockHeight,
    pub finish_by: Option<BlockHeight>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct MinimumDepositResponse {
    pub amount: Coin,
}

impl MinimumDepositResponse {
    pub fn new(amount: Coin) -> Self {
        MinimumDepositResponse { amount }
    }
}
