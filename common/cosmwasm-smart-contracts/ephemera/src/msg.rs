// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "schema")]
use crate::peers::PagedPeerResponse;
use crate::types::JsonPeerInfo;
use cosmwasm_schema::cw_serde;
#[cfg(feature = "schema")]
use cosmwasm_schema::QueryResponses;

#[cw_serde]
pub struct InstantiateMsg {
    pub group_addr: String,
    pub mix_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    RegisterPeer { peer_info: JsonPeerInfo },
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(QueryResponses))]
pub enum QueryMsg {
    #[cfg_attr(feature = "schema", returns(PagedPeerResponse))]
    GetPeers {
        limit: Option<u32>,
        start_after: Option<String>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}
