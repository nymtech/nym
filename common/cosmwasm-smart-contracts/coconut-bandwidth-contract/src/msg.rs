// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{deposit::DepositData, spend_credential::SpendCredentialData};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

#[cfg(feature = "schema")]
use crate::spend_credential::{PagedSpendCredentialResponse, SpendCredentialResponse};
#[cfg(feature = "schema")]
use cosmwasm_schema::QueryResponses;

#[cw_serde]
pub struct InstantiateMsg {
    pub multisig_addr: String,
    pub pool_addr: String,
    pub mix_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    DepositFunds { data: DepositData },
    SpendCredential { data: SpendCredentialData },
    ReleaseFunds { funds: Coin },
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(QueryResponses))]
pub enum QueryMsg {
    #[cfg_attr(feature = "schema", returns(SpendCredentialResponse))]
    GetSpentCredential { blinded_serial_number: String },

    #[cfg_attr(feature = "schema", returns(PagedSpendCredentialResponse))]
    GetAllSpentCredentials {
        limit: Option<u32>,
        start_after: Option<String>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}
