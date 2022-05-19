// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{
    BlockHeight, EncodedBTEPublicKeyWithProof, EncodedEd25519PublicKey, EpochId, NodeIndex,
};
use contracts_common::commitment::ContractSafeCommitment;
use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct DealerDetails {
    pub address: Addr,
    pub joined_at: BlockHeight,
    pub left_at: Option<BlockHeight>,
    pub blacklisting: Option<Blacklisting>,
    pub ed25519_public_key: EncodedEd25519PublicKey,
    pub bte_public_key_with_proof: EncodedBTEPublicKeyWithProof,
    pub assigned_index: NodeIndex,
    // TODO: in the future, perhaps, this could get replaced by some gossip system and address books
    // like in 'normal' Tendermint?
    pub host: String,
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
                "blacklisted at block height {}. reason given: {}. expires at: {}",
                self.height, self.height, expiration
            )
        } else {
            write!(
                f,
                "permanently blacklisted at block height {}. reason given: {}",
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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DealerType {
    Current,
    Past,
    Unknown,
}

impl DealerType {
    pub fn is_current(&self) -> bool {
        matches!(&self, DealerType::Current)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DealerDetailsResponse {
    pub details: Option<DealerDetails>,
    pub dealer_type: DealerType,
}

impl DealerDetailsResponse {
    pub fn new(details: Option<DealerDetails>, dealer_type: DealerType) -> Self {
        DealerDetailsResponse {
            details,
            dealer_type,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PagedDealerResponse {
    pub dealers: Vec<DealerDetails>,
    pub per_page: usize,
    pub start_next_after: Option<Addr>,
}

impl PagedDealerResponse {
    pub fn new(
        dealers: Vec<DealerDetails>,
        per_page: usize,
        start_next_after: Option<Addr>,
    ) -> Self {
        PagedDealerResponse {
            dealers,
            per_page,
            start_next_after,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PagedBlacklistingResponse {
    pub blacklisted_dealers: Vec<BlacklistedDealer>,
    pub per_page: usize,
    pub start_next_after: Option<Addr>,
}

impl PagedBlacklistingResponse {
    pub fn new(
        blacklisted_dealers: Vec<BlacklistedDealer>,
        per_page: usize,
        start_next_after: Option<Addr>,
    ) -> Self {
        PagedBlacklistingResponse {
            blacklisted_dealers,
            per_page,
            start_next_after,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BlacklistedDealer {
    pub dealer: Addr,
    pub blacklisting: Blacklisting,
}

impl BlacklistedDealer {
    pub fn new(dealer: Addr, blacklisting: Blacklisting) -> Self {
        BlacklistedDealer {
            dealer,
            blacklisting,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BlacklistingResponse {
    pub dealer: Addr,
    pub blacklisting: Option<Blacklisting>,
}

impl BlacklistingResponse {
    pub fn new(dealer: Addr, blacklisting: Option<Blacklisting>) -> Self {
        BlacklistingResponse {
            dealer,
            blacklisting,
        }
    }

    pub fn is_blacklisted(&self, current_height: BlockHeight) -> bool {
        match self.blacklisting {
            None => false,
            Some(blacklisting) => !blacklisting.has_expired(current_height),
        }
    }

    pub fn unchecked_get_blacklisting(&self) -> &Blacklisting {
        self.blacklisting
            .as_ref()
            .expect("dealer is not blacklisted")
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ContractDealingCommitment {
    pub commitment: ContractSafeCommitment,
    pub dealer: Addr,
    pub epoch_id: EpochId,
}

impl ContractDealingCommitment {
    pub fn new(commitment: ContractSafeCommitment, dealer: Addr, epoch_id: EpochId) -> Self {
        ContractDealingCommitment {
            commitment,
            dealer,
            epoch_id,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PagedCommitmentsResponse {
    pub commitments: Vec<ContractDealingCommitment>,
    pub per_page: usize,
    pub start_next_after: Option<Addr>,
}

impl PagedCommitmentsResponse {
    pub fn new(
        commitments: Vec<ContractDealingCommitment>,
        per_page: usize,
        start_next_after: Option<Addr>,
    ) -> Self {
        PagedCommitmentsResponse {
            commitments,
            per_page,
            start_next_after,
        }
    }
}
