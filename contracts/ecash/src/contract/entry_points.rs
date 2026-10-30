// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! CosmWasm entry points for the ecash contract.

use crate::contract::NymEcashContract;
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
};
use nym_ecash_contract_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use nym_ecash_contract_common::EcashContractError;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, EcashContractError> {
    NymEcashContract::new().instantiate(deps, info, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, EcashContractError> {
    let contract = NymEcashContract::new();
    match msg {
        ExecuteMsg::DepositTicketBookFunds { identity_key } => {
            contract.deposit_ticket_book_funds(deps, info, identity_key)
        }
        ExecuteMsg::RequestRedemption {
            commitment_bs58,
            number_of_tickets,
        } => contract.request_redemption(deps, env, info, commitment_bs58, number_of_tickets),
        ExecuteMsg::RedeemTickets { n, gw } => contract.redeem_tickets(deps, info, n, gw),
        ExecuteMsg::UpdateAdmin { admin } => contract.update_admin(deps, info, admin),
        ExecuteMsg::UpdateDefaultDepositValue { new_deposit } => {
            contract.update_default_deposit_value(deps, info, new_deposit)
        }
        ExecuteMsg::SetReducedDepositPrice { address, deposit } => {
            contract.set_reduced_deposit_price(deps, info, address, deposit)
        }
        ExecuteMsg::RemoveReducedDepositPrice { address } => {
            contract.remove_reduced_deposit_price(deps, info, address)
        }
        ExecuteMsg::ProposeToBlacklist { public_key } => {
            contract.propose_to_blacklist(deps, env, info, public_key)
        }
        ExecuteMsg::AddToBlacklist { public_key } => {
            contract.add_to_blacklist(deps, env, info, public_key)
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, EcashContractError> {
    let contract = NymEcashContract::new();
    match msg {
        QueryMsg::GetBlacklistedAccount { public_key } => Ok(to_json_binary(
            &contract.get_blacklisted_account(deps, public_key)?,
        )?),
        QueryMsg::GetBlacklistPaged { limit, start_after } => Ok(to_json_binary(
            &contract.get_blacklist_paged(deps, limit, start_after)?,
        )?),
        QueryMsg::GetDefaultDepositAmount {} => {
            Ok(to_json_binary(&contract.get_default_deposit_amount(deps)?)?)
        }
        QueryMsg::GetReducedDepositAmount { address } => Ok(to_json_binary(
            &contract.get_reduced_deposit_amount(deps, address)?,
        )?),
        QueryMsg::GetAllWhitelistedAccounts {} => Ok(to_json_binary(
            &contract.get_all_whitelisted_accounts(deps)?,
        )?),
        QueryMsg::GetDeposit { deposit_id } => {
            Ok(to_json_binary(&contract.get_deposit(deps, deposit_id)?)?)
        }
        QueryMsg::GetLatestDeposit {} => Ok(to_json_binary(&contract.get_latest_deposit(deps)?)?),
        QueryMsg::GetDepositsPaged { limit, start_after } => Ok(to_json_binary(
            &contract.get_deposits_paged(deps, limit, start_after)?,
        )?),
        QueryMsg::GetDepositsStatistics {} => {
            Ok(to_json_binary(&contract.get_deposits_statistics(deps)?)?)
        }
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, EcashContractError> {
    NymEcashContract::new().migrate(deps, msg)
}

#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, EcashContractError> {
    NymEcashContract::new().reply(deps, msg)
}
