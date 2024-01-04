// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{
    DealingIndex, EncodedBTEPublicKeyWithProof, EpochId, PartialContractDealing, TimeConfiguration,
};
use crate::verification_key::VerificationKeyShare;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cfg(feature = "schema")]
use crate::{
    dealer::{
        DealerDetailsResponse, DealingResponse, DealingStatusResponse, PagedDealerResponse,
        PagedDealingsResponse,
    },
    types::{Epoch, InitialReplacementData, State},
    verification_key::PagedVKSharesResponse,
};
#[cfg(feature = "schema")]
use cosmwasm_schema::QueryResponses;

#[cw_serde]
pub struct InstantiateMsg {
    pub group_addr: String,
    pub multisig_addr: String,
    pub time_configuration: Option<TimeConfiguration>,
    pub mix_denom: String,

    /// Specifies the number of elements in the derived keys
    pub key_size: u32,
}

#[cw_serde]
pub enum ExecuteMsg {
    RegisterDealer {
        bte_key_with_proof: EncodedBTEPublicKeyWithProof,
        announce_address: String,
        resharing: bool,
    },

    CommitDealing {
        dealing: PartialContractDealing,
        resharing: bool,
    },

    CommitVerificationKeyShare {
        share: VerificationKeyShare,
        resharing: bool,
    },

    VerifyVerificationKeyShare {
        // TODO: this should be using a String...
        owner: Addr,
        resharing: bool,
    },

    SurpassedThreshold {},

    AdvanceEpochState {},
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(QueryResponses))]
pub enum QueryMsg {
    #[cfg_attr(feature = "schema", returns(State))]
    GetState {},

    #[cfg_attr(feature = "schema", returns(Epoch))]
    GetCurrentEpochState {},

    #[cfg_attr(feature = "schema", returns(u64))]
    GetCurrentEpochThreshold {},

    #[cfg_attr(feature = "schema", returns(Option<InitialReplacementData>))]
    GetInitialDealers {},

    #[cfg_attr(feature = "schema", returns(DealerDetailsResponse))]
    GetDealerDetails { dealer_address: String },

    #[cfg_attr(feature = "schema", returns(PagedDealerResponse))]
    GetCurrentDealers {
        limit: Option<u32>,
        start_after: Option<String>,
    },

    #[cfg_attr(feature = "schema", returns(PagedDealerResponse))]
    GetPastDealers {
        limit: Option<u32>,
        start_after: Option<String>,
    },

    #[cfg_attr(feature = "schema", returns(DealingStatusResponse))]
    GetDealingStatus {
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    },

    #[cfg_attr(feature = "schema", returns(DealingResponse))]
    GetDealing {
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    },

    #[cfg_attr(feature = "schema", returns(PagedDealingsResponse))]
    GetDealings {
        epoch_id: EpochId,
        dealer: String,
        limit: Option<u32>,
        start_after: Option<DealingIndex>,
    },

    #[cfg_attr(feature = "schema", returns(PagedVKSharesResponse))]
    GetVerificationKeys {
        epoch_id: EpochId,
        limit: Option<u32>,
        start_after: Option<String>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}
