// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{ContractSafeBytes, EncodedBTEPublicKeyWithProof, NodeIndex};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub struct DealerDetails {
    pub address: Addr,
    pub bte_public_key_with_proof: EncodedBTEPublicKeyWithProof,
    pub announce_address: String,
    pub assigned_index: NodeIndex,
}

#[cw_serde]
#[derive(Copy)]
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

#[cw_serde]
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

#[cw_serde]
pub struct PagedDealerResponse {
    pub dealers: Vec<DealerDetails>,
    pub per_page: usize,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
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

#[cw_serde]
pub struct ContractDealing {
    pub dealing: ContractSafeBytes,
    pub dealer: Addr,
}

impl ContractDealing {
    pub fn new(dealing: ContractSafeBytes, dealer: Addr) -> Self {
        ContractDealing { dealing, dealer }
    }
}

#[cw_serde]
pub struct PagedDealingsResponse {
    pub dealings: Vec<ContractDealing>,
    pub per_page: usize,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
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
