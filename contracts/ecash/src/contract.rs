// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! CosmWasm entry points for the ecash contract.

use crate::constants::{BLACKLIST_PROPOSAL_REPLY_ID, REDEMPTION_PROPOSAL_REPLY_ID};
use crate::helpers::Config;
use crate::queries::{
    query_all_whitelisted_accounts, query_blacklist_paged, query_blacklisted_account,
    query_default_deposit_amount, query_deposit, query_deposits_paged, query_deposits_statistics,
    query_latest_deposit, query_reduced_deposit_amount,
};
use crate::storage::ECASH_CONTRACT_STORAGE;
use crate::transactions::{
    try_add_to_blacklist, try_deposit_ticket_book_funds, try_handle_blacklist_proposal_reply,
    try_handle_redemption_proposal_reply, try_propose_to_blacklist, try_redeem_tickets,
    try_remove_reduced_deposit_price, try_request_redemption, try_set_reduced_deposit_price,
    try_update_admin, try_update_default_deposit_value,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
};
use cw4::Cw4Contract;
use nym_contracts_common::set_build_information;
use nym_ecash_contract_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use nym_ecash_contract_common::EcashContractError;

const CONTRACT_NAME: &str = "crate:nym-ecash";

/// Contract semver, taken from `Cargo.toml` at build time; recorded in cw2
/// storage so migrations can detect the source version.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One-shot contract setup. Validates the cross-contract pointers, records
/// the cw2 version and build metadata, and initialises storage with the
/// sender as the contract admin.
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, EcashContractError> {
    let multisig_addr = deps.api.addr_validate(&msg.multisig_addr)?;
    let holding_account = deps.api.addr_validate(&msg.holding_account)?;
    let group_addr = Cw4Contract(deps.api.addr_validate(&msg.group_addr).map_err(|_| {
        EcashContractError::InvalidGroup {
            addr: msg.group_addr.clone(),
        }
    })?);

    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    ECASH_CONTRACT_STORAGE.initialise(
        deps,
        info.sender,
        multisig_addr,
        Config {
            group_addr,
            holding_account,
            deposit_amount: msg.deposit_amount,
        },
    )?;

    Ok(Response::default())
}

/// State-mutating dispatcher. Concrete handlers live in [`crate::transactions`].
#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, EcashContractError> {
    match msg {
        ExecuteMsg::DepositTicketBookFunds { identity_key } => {
            try_deposit_ticket_book_funds(deps, info, identity_key)
        }
        ExecuteMsg::RequestRedemption {
            commitment_bs58,
            number_of_tickets,
        } => try_request_redemption(deps, env, info, commitment_bs58, number_of_tickets),
        ExecuteMsg::RedeemTickets { n, gw } => try_redeem_tickets(deps, info, n, gw),
        ExecuteMsg::UpdateAdmin { admin } => try_update_admin(deps, info, admin),
        ExecuteMsg::UpdateDefaultDepositValue { new_deposit } => {
            try_update_default_deposit_value(deps, info, new_deposit)
        }
        ExecuteMsg::SetReducedDepositPrice { address, deposit } => {
            try_set_reduced_deposit_price(deps, info, address, deposit)
        }
        ExecuteMsg::RemoveReducedDepositPrice { address } => {
            try_remove_reduced_deposit_price(deps, info, address)
        }
        ExecuteMsg::ProposeToBlacklist { public_key } => {
            try_propose_to_blacklist(deps, env, info, public_key)
        }
        ExecuteMsg::AddToBlacklist { public_key } => {
            try_add_to_blacklist(deps, env, info, public_key)
        }
    }
}

/// Read-only dispatcher. Concrete handlers live in [`crate::queries`].
#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, EcashContractError> {
    match msg {
        QueryMsg::GetBlacklistedAccount { public_key } => Ok(to_json_binary(
            &query_blacklisted_account(deps, public_key)?,
        )?),
        QueryMsg::GetBlacklistPaged { limit, start_after } => Ok(to_json_binary(
            &query_blacklist_paged(deps, limit, start_after)?,
        )?),
        QueryMsg::GetDefaultDepositAmount {} => {
            Ok(to_json_binary(&query_default_deposit_amount(deps)?)?)
        }
        QueryMsg::GetReducedDepositAmount { address } => Ok(to_json_binary(
            &query_reduced_deposit_amount(deps, address)?,
        )?),
        QueryMsg::GetAllWhitelistedAccounts {} => {
            Ok(to_json_binary(&query_all_whitelisted_accounts(deps)?)?)
        }
        QueryMsg::GetDeposit { deposit_id } => {
            Ok(to_json_binary(&query_deposit(deps, deposit_id)?)?)
        }
        QueryMsg::GetLatestDeposit {} => Ok(to_json_binary(&query_latest_deposit(deps)?)?),
        QueryMsg::GetDepositsPaged { limit, start_after } => Ok(to_json_binary(
            &query_deposits_paged(deps, limit, start_after)?,
        )?),
        QueryMsg::GetDepositsStatistics {} => {
            Ok(to_json_binary(&query_deposits_statistics(deps)?)?)
        }
    }
}

/// Migration entry point.
///
/// Refreshes recorded build information and ensures the existing on-chain
/// contract version is at most the current `CONTRACT_VERSION` (i.e. forbids
/// downgrades). Any data migrations are dispatched via
/// [`crate::queued_migrations`].
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, EcashContractError> {
    set_build_information!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}

/// Dispatch reply messages by id. Surfaces `InvalidReplyId` for any id
/// not matching `BLACKLIST_PROPOSAL_REPLY_ID` or
/// `REDEMPTION_PROPOSAL_REPLY_ID`.
#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, EcashContractError> {
    match msg.id {
        n if n == BLACKLIST_PROPOSAL_REPLY_ID => try_handle_blacklist_proposal_reply(deps, msg),
        n if n == REDEMPTION_PROPOSAL_REPLY_ID => try_handle_redemption_proposal_reply(msg),
        other => Err(EcashContractError::InvalidReplyId { id: other }),
    }
}
