// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

#[cfg(feature = "schema")]
use crate::blacklist::{BlacklistedAccountResponse, PagedBlacklistedAccountResponse};
#[cfg(feature = "schema")]
use crate::deposit::{DepositResponse, LatestDepositResponse, PagedDepositsResponse};
#[cfg(feature = "schema")]
use crate::deposit_statistics::DepositsStatistics;
#[cfg(feature = "schema")]
use crate::reduced_deposit::WhitelistedAccountsResponse;
#[cfg(feature = "schema")]
use cosmwasm_schema::QueryResponses;

/// Instantiation payload. The sender of the instantiate transaction becomes the
/// contract admin; the three addresses are bech32-validated and persisted as
/// immutable cross-contract pointers (see spec requirement "Contract instantiation").
#[cw_serde]
pub struct InstantiateMsg {
    /// Cosmos SDK address reserved for the future pool-contract transition.
    /// Stored in `Config` but never debited by the current contract.
    pub holding_account: String,

    /// cw3 multisig contract that gates `RedeemTickets` and (in the redesign)
    /// blacklist proposals. Not updatable through any execute path.
    pub multisig_addr: String,

    /// cw4 group contract referenced by the (stubbed) blacklist proposal flow.
    pub group_addr: String,

    /// Default per-deposit price. The denom of this coin is the contract's
    /// canonical denom for the rest of its lifetime.
    pub deposit_amount: Coin,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Submitted by clients to escrow funds and register a claimed ed25519
    /// identity key. Mints a sequential `deposit_id`. The contract does not
    /// verify control of the identity key - that proof is checked off-chain by
    /// nym-api signers at blind-sign time.
    DepositTicketBookFunds { identity_key: String },

    /// Submitted by gateways to request batch redemption of spent tickets.
    /// Dispatches a `Propose` SubMsg to the multisig contract; the actual
    /// transfer effect is gated behind multisig approval.
    RequestRedemption {
        commitment_bs58: String,
        number_of_tickets: u16,
    },

    /// **Legacy / dead code.** Only callable by the multisig; bumps the
    /// unredeemed-tickets counter and emits a `ticket_redemption` event with
    /// `moved_to_holding_account = "false"`. No known consumer depends on the
    /// side effects; candidate for removal in a follow-on breaking-schema
    /// change.
    RedeemTickets { n: u16, gw: String },

    /// Transfer the contract admin role. Only the current admin may sign.
    /// Dispatches via the cw_controllers `execute_update_admin` handshake.
    UpdateAdmin { admin: String },

    /// Overwrite `Config::deposit_amount`. Only callable by the contract admin.
    /// Rejects values below `nym_network_defaults::TICKETBOOK_SIZE` and trips
    /// `TicketBookSizeChanged` if the snapshotted invariant has diverged from
    /// the current crate constant.
    #[serde(alias = "update_deposit_value")]
    UpdateDefaultDepositValue { new_deposit: Coin },

    /// Set (or overwrite) a reduced deposit price for a specific address.
    /// Only callable by the contract admin.
    SetReducedDepositPrice { address: String, deposit: Coin },

    /// Remove the reduced deposit price for a specific address, reverting them to
    /// the default price. Returns an error if the address has no custom price set.
    /// Only callable by the contract admin.
    RemoveReducedDepositPrice { address: String },

    /// **Stubbed**: always returns `EcashContractError::UnimplementedBlacklisting`.
    /// Storage, reply handler, and helper paths exist but are unreachable from
    /// the public ExecuteMsg surface. Preserved for the redesign.
    ProposeToBlacklist { public_key: String },

    /// **Stubbed**: always returns `EcashContractError::UnimplementedBlacklisting`.
    AddToBlacklist { public_key: String },
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(QueryResponses))]
pub enum QueryMsg {
    /// Look up a blacklist entry by its bs58-encoded ed25519 public key. Always
    /// returns `None` on a freshly deployed contract because the blacklist
    /// execute paths are stubbed.
    #[cfg_attr(feature = "schema", returns(BlacklistedAccountResponse))]
    GetBlacklistedAccount { public_key: String },

    /// Paginated listing of blacklist entries. Always empty today (see stubbed
    /// blacklist surface). Defaults: limit 50, max 75.
    #[cfg_attr(feature = "schema", returns(PagedBlacklistedAccountResponse))]
    GetBlacklistPaged {
        limit: Option<u32>,
        start_after: Option<String>,
    },

    /// Default per-deposit price (`Config::deposit_amount`). The
    /// `GetRequiredDepositAmount` aliases are kept for backwards compatibility.
    #[cfg_attr(feature = "schema", returns(Coin))]
    #[serde(alias = "get_required_deposit_amount")]
    #[serde(alias = "GetRequiredDepositAmount")]
    GetDefaultDepositAmount {},

    /// Per-address reduced deposit price override, if any. `None` for any
    /// non-whitelisted address.
    #[cfg_attr(feature = "schema", returns(Option<Coin>))]
    GetReducedDepositAmount { address: String },

    /// Enumerate every reduced-deposit whitelist entry in ascending address
    /// order. Unpaginated by design (the whitelist is expected to stay small).
    #[cfg_attr(feature = "schema", returns(WhitelistedAccountsResponse))]
    GetAllWhitelistedAccounts {},

    /// Look up a deposit by id. Returns `{ id, deposit: None }` when the id has
    /// not yet been assigned.
    #[cfg_attr(feature = "schema", returns(DepositResponse))]
    GetDeposit { deposit_id: u32 },

    /// Most recently assigned deposit (or `{ deposit: None }` on a fresh
    /// contract). See `DepositStorage::latest_deposit`.
    #[cfg_attr(feature = "schema", returns(LatestDepositResponse))]
    GetLatestDeposit {},

    /// Paginated listing of deposits in ascending id order. Defaults: limit 50,
    /// max 100.
    #[cfg_attr(feature = "schema", returns(PagedDepositsResponse))]
    GetDepositsPaged {
        limit: Option<u32>,
        start_after: Option<u32>,
    },

    /// Aggregate statistics: global totals + per-account custom-price
    /// breakdowns. Reassembled in a single read pass from `PoolCounters` and
    /// `DepositStatsStorage`.
    #[cfg_attr(feature = "schema", returns(DepositsStatistics))]
    GetDepositsStatistics {},
}

#[cw_serde]
pub struct MigrateMsg {
    /// Initial set of whitelisted accounts with their reduced deposit prices.
    /// Each entry is validated and stored during migration.
    pub initial_whitelist: Vec<WhitelistedDeposit>,
}

/// An address and its reduced deposit price, used when seeding the whitelist
/// via migration.
#[cw_serde]
pub struct WhitelistedDeposit {
    pub address: String,
    pub deposit: Coin,
}
