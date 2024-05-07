// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;

#[cfg(feature = "schema")]
use crate::blacklist::BlacklistedAccountResponse;
#[cfg(feature = "schema")]
use cosmwasm_schema::QueryResponses;

#[cw_serde]
pub struct InstantiateMsg {
    pub multisig_addr: String,
    pub group_addr: String,
    pub mix_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    DepositFunds {
        deposit_info: String,
        identity_key: String,
    },
    PrepareCredential {
        serial_number: String,
        gateway_cosmos_address: String,
    },
    SpendCredential {
        serial_number: String,
        gateway_cosmos_address: String,
    },
    ProposeToBlacklist {
        public_key: String,
    },
    AddToBlacklist {
        public_key: String,
    },
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(QueryResponses))]
pub enum QueryMsg {
    #[cfg_attr(feature = "schema", returns(BlacklistedAccountResponse))]
    GetBlacklistedAccount { public_key: String },

    #[cfg_attr(feature = "schema", returns(SpendCredentialResponse))]
    GetSpentCredential { serial_number: String },

    #[cfg_attr(feature = "schema", returns(PagedSpendCredentialResponse))]
    GetAllSpentCredentialsPaged {
        limit: Option<u32>,
        start_after: Option<String>,
    },
}