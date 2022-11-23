// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub use crate::dealer::{DealerDetails, PagedDealerResponse};
pub use contracts_common::dealings::ContractSafeBytes;
pub use cosmwasm_std::{Addr, Coin};

pub type EncodedBTEPublicKeyWithProof = String;
pub type EncodedBTEPublicKeyWithProofRef<'a> = &'a str;
pub type NodeIndex = u64;

// 2 public attributes, 2 private attributes, 1 fixed for coconut credential
pub const TOTAL_DEALINGS: usize = 2 + 2 + 1;

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
    VerificationKeySubmission,
    VerificationKeyValidation,
    VerificationKeyFinalization,
    InProgress,
}

impl Default for EpochState {
    fn default() -> Self {
        Self::PublicKeySubmission
    }
}

impl Display for EpochState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EpochState::PublicKeySubmission => write!(f, "PublicKeySubmission"),
            EpochState::DealingExchange => write!(f, "DealingExchange"),
            EpochState::VerificationKeySubmission => write!(f, "VerificationKeySubmission"),
            EpochState::VerificationKeyValidation => write!(f, "VerificationKeyValidation"),
            EpochState::VerificationKeyFinalization => write!(f, "VerificationKeyFinalization"),
            EpochState::InProgress => write!(f, "InProgress"),
        }
    }
}

impl EpochState {
    pub fn next(self) -> Option<Self> {
        match self {
            EpochState::PublicKeySubmission => Some(EpochState::DealingExchange),
            EpochState::DealingExchange => Some(EpochState::VerificationKeySubmission),
            EpochState::VerificationKeySubmission => Some(EpochState::VerificationKeyValidation),
            EpochState::VerificationKeyValidation => Some(EpochState::VerificationKeyFinalization),
            EpochState::VerificationKeyFinalization => Some(EpochState::InProgress),
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
