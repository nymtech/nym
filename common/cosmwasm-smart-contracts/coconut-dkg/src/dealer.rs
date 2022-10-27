// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{ContractSafeBytes, EncodedBTEPublicKeyWithProof, NodeIndex, TOTAL_DEALINGS};
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
pub struct ContractDealing {
    pub dealings: [ContractSafeBytes; TOTAL_DEALINGS],
    pub dealer: Addr,
}

impl ContractDealing {
    pub fn new(dealings: [ContractSafeBytes; TOTAL_DEALINGS], dealer: Addr) -> Self {
        ContractDealing { dealings, dealer }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PagedDealingsResponse {
    pub dealings: Vec<ContractDealing>,
    pub per_page: usize,
    pub start_next_after: Option<Addr>,
}

impl PagedDealingsResponse {
    pub fn new(
        dealings: Vec<ContractDealing>,
        per_page: usize,
        start_next_after: Option<Addr>,
    ) -> Self {
        PagedDealingsResponse {
            dealings,
            per_page,
            start_next_after,
        }
    }
}
