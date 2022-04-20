// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub type BlockHeight = u64;
pub type EncodedEd25519PublicKey = String;
pub type EncodedEd25519PublicKeyRef<'a> = &'a str;
pub type EncodedBTEPublicKeyWithProof = String;
pub type NodeIndex = u64;
pub type EpochId = u64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct DealerDetails {
    pub joined_at: BlockHeight,
    pub left_at: Option<BlockHeight>,
    pub blacklisting: Option<Blacklisting>,
    pub ed25519_public_key: EncodedEd25519PublicKey,
    pub bte_public_key_with_proof: EncodedBTEPublicKeyWithProof,
    pub assigned_index: NodeIndex,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Blacklisting {
    pub reason: BlacklistingReason,
    pub height: BlockHeight,
    pub expiration: Option<BlockHeight>,
}

impl Blacklisting {
    pub fn has_expired(&self, current_block: BlockHeight) -> bool {
        self.expiration
            .map(|expiration| expiration <= current_block)
            .unwrap_or_default()
    }
}

impl Display for Blacklisting {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(expiration) = self.expiration {
            write!(
                f,
                "blacklisted at block height {}. reason given: {}. Expires at: {}",
                self.height, self.height, expiration
            )
        } else {
            write!(
                f,
                "blacklisted at block height {}. reason given: {}",
                self.height, self.height
            )
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BlacklistingReason {
    InactiveForConsecutiveEpochs,
    MalformedBTEPublicKey,
    InvalidBTEPublicKey,
    MalformedEd25519PublicKey,
    Ed25519PossessionVerificationFailure,
}

impl Display for BlacklistingReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BlacklistingReason::InactiveForConsecutiveEpochs => {
                write!(f, "has been inactive for multiple consecutive epochs")
            }
            BlacklistingReason::MalformedBTEPublicKey => {
                write!(f, "provided malformed BTE Public Key")
            }
            BlacklistingReason::InvalidBTEPublicKey => write!(f, "provided invalid BTE Public Key"),
            BlacklistingReason::MalformedEd25519PublicKey => {
                write!(f, "provided malformed ed25519 Public Key")
            }
            BlacklistingReason::Ed25519PossessionVerificationFailure => {
                write!(
                    f,
                    "failed to verify possession of provided ed25519 Public Key"
                )
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Epoch {
    pub id: EpochId,
    pub state: EpochState,
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

        Some(Epoch { id: self.id, state })
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
