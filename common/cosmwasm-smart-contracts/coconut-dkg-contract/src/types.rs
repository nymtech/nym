// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Epoch {
    pub id: EpochId,
    pub state: EpochState,

    // TODO: need to ponder a bit whether it's actually a property of a particular epoch
    pub system_threshold: Threshold,
}

impl Epoch {
    pub fn next_state(
        &self,
        current_time: Option<BlockHeight>,
        end_time: Option<BlockHeight>,
    ) -> Option<Self> {
        let state = match self.state {
            EpochState::PublicKeySubmission { finish_by, .. } => EpochState::DealingExchange {
                begun_at: finish_by,
                finish_by: end_time?,
            },
            EpochState::DealingExchange { finish_by, .. } => EpochState::ComplaintSubmission {
                begun_at: finish_by,
                finish_by: end_time?,
            },
            EpochState::ComplaintSubmission { finish_by, .. } => EpochState::ComplaintVoting {
                begun_at: finish_by,
                finish_by: end_time?,
            },
            EpochState::ComplaintVoting { finish_by, .. } => {
                EpochState::VerificationKeySubmission {
                    begun_at: finish_by,
                    finish_by: end_time?,
                }
            }
            EpochState::VerificationKeySubmission { finish_by, .. } => {
                EpochState::VerificationKeyMismatchSubmission {
                    begun_at: finish_by,
                    finish_by: end_time?,
                }
            }
            EpochState::VerificationKeyMismatchSubmission { finish_by, .. } => {
                EpochState::VerificationKeyMismatchVoting {
                    begun_at: finish_by,
                    finish_by: end_time?,
                }
            }
            EpochState::VerificationKeyMismatchVoting { finish_by, .. } => EpochState::InProgress {
                begun_at: finish_by,
                finish_by: end_time,
            },
            EpochState::InProgress { .. } => EpochState::PublicKeySubmission {
                begun_at: current_time?,
                finish_by: end_time?,
            },
        };

        Some(Epoch {
            id: self.id,
            state,
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
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EpochState {
    PublicKeySubmission {
        begun_at: BlockHeight,
        finish_by: BlockHeight,
    },
    DealingExchange {
        begun_at: BlockHeight,
        finish_by: BlockHeight,
    },
    ComplaintSubmission {
        begun_at: BlockHeight,
        finish_by: BlockHeight,
    },
    ComplaintVoting {
        begun_at: BlockHeight,
        finish_by: BlockHeight,
    },
    VerificationKeySubmission {
        begun_at: BlockHeight,
        finish_by: BlockHeight,
    },
    VerificationKeyMismatchSubmission {
        begun_at: BlockHeight,
        finish_by: BlockHeight,
    },
    VerificationKeyMismatchVoting {
        begun_at: BlockHeight,
        finish_by: BlockHeight,
    },
    InProgress {
        begun_at: BlockHeight,
        // not entirely sure about that one yet. we'll see how it works out when we get to epoch transition
        finish_by: Option<BlockHeight>,
    },
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
