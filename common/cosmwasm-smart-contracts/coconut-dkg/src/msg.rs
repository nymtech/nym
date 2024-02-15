// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealing::{DealingChunkInfo, PartialContractDealing};
use crate::types::{
    ChunkIndex, DealingIndex, EncodedBTEPublicKeyWithProof, EpochId, TimeConfiguration,
};
use crate::verification_key::VerificationKeyShare;
use contracts_common::IdentityKey;
use cosmwasm_schema::cw_serde;

#[cfg(feature = "schema")]
use crate::{
    dealer::{
        DealerDetailsResponse, PagedDealerIndexResponse, PagedDealerResponse,
        RegisteredDealerDetails,
    },
    dealing::{
        DealerDealingsStatusResponse, DealingChunkResponse, DealingChunkStatusResponse,
        DealingMetadataResponse, DealingStatusResponse,
    },
    types::{Epoch, State},
    verification_key::{PagedVKSharesResponse, VkShareResponse},
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
    // we could have just re-used AdvanceEpochState, but imo an explicit message is better
    InitiateDkg {},

    RegisterDealer {
        bte_key_with_proof: EncodedBTEPublicKeyWithProof,
        identity_key: IdentityKey,
        announce_address: String,
        resharing: bool,
    },

    CommitDealingsMetadata {
        dealing_index: DealingIndex,
        chunks: Vec<DealingChunkInfo>,
        resharing: bool,
    },

    CommitDealingsChunk {
        chunk: PartialContractDealing,
    },

    CommitVerificationKeyShare {
        share: VerificationKeyShare,
        resharing: bool,
    },

    VerifyVerificationKeyShare {
        owner: String,
        resharing: bool,
    },

    AdvanceEpochState {},

    TriggerReset {},

    TriggerResharing {},
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

    #[cfg_attr(feature = "schema", returns(RegisteredDealerDetails))]
    GetRegisteredDealer {
        dealer_address: String,
        epoch_id: Option<EpochId>,
    },

    #[cfg_attr(feature = "schema", returns(DealerDetailsResponse))]
    GetDealerDetails { dealer_address: String },

    #[cfg_attr(feature = "schema", returns(PagedDealerResponse))]
    GetCurrentDealers {
        limit: Option<u32>,
        start_after: Option<String>,
    },

    #[cfg_attr(feature = "schema", returns(PagedDealerIndexResponse))]
    GetDealerIndices {
        limit: Option<u32>,
        start_after: Option<String>,
    },

    #[cfg_attr(feature = "schema", returns(DealingMetadataResponse))]
    GetDealingsMetadata {
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    },

    #[cfg_attr(feature = "schema", returns(DealerDealingsStatusResponse))]
    GetDealerDealingsStatus { epoch_id: EpochId, dealer: String },

    #[cfg_attr(feature = "schema", returns(DealingStatusResponse))]
    GetDealingStatus {
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    },

    #[cfg_attr(feature = "schema", returns(DealingChunkStatusResponse))]
    GetDealingChunkStatus {
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    },

    #[cfg_attr(feature = "schema", returns(DealingChunkResponse))]
    GetDealingChunk {
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    },

    #[cfg_attr(feature = "schema", returns(VkShareResponse))]
    GetVerificationKey { epoch_id: EpochId, owner: String },

    #[cfg_attr(feature = "schema", returns(PagedVKSharesResponse))]
    GetVerificationKeys {
        epoch_id: EpochId,
        limit: Option<u32>,
        start_after: Option<String>,
    },

    /// Gets the stored contract version information that's required by the CW2 spec interface for migrations.
    #[serde(rename = "get_cw2_contract_version")]
    #[cfg_attr(feature = "schema", returns(cw2::ContractVersion))]
    GetCW2ContractVersion {},
}

#[cw_serde]
pub struct MigrateMsg {}
