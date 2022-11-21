// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{ContractSafeCommitment, EncodedBTEPublicKeyWithProof, NodeIndex};
use cosmwasm_std::{Addr, Coin};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct DealerDetails {
    pub address: Addr,
    pub bte_public_key_with_proof: EncodedBTEPublicKeyWithProof,
    pub assigned_index: NodeIndex,
    pub deposit: Coin,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ContractDealingCommitment {
    pub commitment: ContractSafeCommitment,
    pub dealer: Addr,
}

impl ContractDealingCommitment {
    pub fn new(commitment: ContractSafeCommitment, dealer: Addr) -> Self {
        ContractDealingCommitment { commitment, dealer }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
