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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EpochState {
    InProgress {
        begun_at: BlockHeight,
        // not entirely sure about that one yet. we'll see how it works out when we get to epoch transition
        finish_by: Option<BlockHeight>,
    },
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
}
