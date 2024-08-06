// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

#[cfg(feature = "schema")]
use crate::blacklist::{BlacklistedAccountResponse, PagedBlacklistedAccountResponse};
#[cfg(feature = "schema")]
use crate::deposit::{DepositResponse, PagedDepositsResponse};
#[cfg(feature = "schema")]
use cosmwasm_schema::QueryResponses;

#[cw_serde]
pub struct InstantiateMsg {
    pub holding_account: String,
    pub multisig_addr: String,
    pub group_addr: String,
    pub deposit_amount: Coin,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Used by clients to request ticket books from the signers
    DepositTicketBookFunds {
        identity_key: String,
    },

    /// Used by gateways to batch redeem tokens from the spent tickets
    RequestRedemption {
        commitment_bs58: String,
        number_of_tickets: u16,
    },

    /// The actual message that gets executed, after multisig votes, that transfers the ticket tokens into gateway's (and the holding) account
    RedeemTickets {
        n: u16,
        gw: String,
    },

    UpdateAdmin {
        admin: String,
    },

    UpdateDepositValue {
        new_deposit: Coin,
    },

    // TODO: properly implement
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

    #[cfg_attr(feature = "schema", returns(PagedBlacklistedAccountResponse))]
    GetBlacklistPaged {
        limit: Option<u32>,
        start_after: Option<String>,
    },

    #[cfg_attr(feature = "schema", returns(Coin))]
    GetRequiredDepositAmount {},

    #[cfg_attr(feature = "schema", returns(DepositResponse))]
    GetDeposit { deposit_id: u32 },

    #[cfg_attr(feature = "schema", returns(PagedDepositsResponse))]
    GetDepositsPaged {
        limit: Option<u32>,
        start_after: Option<u32>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}
