// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::MINIMUM_DEPOSIT;
use crate::dealers::queries::{
    query_current_dealers_paged, query_dealer_details, query_past_dealers_paged,
};
use crate::dealings::queries::query_dealings_paged;
use crate::epoch_state::queries::query_current_epoch_state;
use crate::error::ContractError;
use crate::state::{State, ADMIN, STATE};
use crate::verification_key_shares::queries::query_vk_shares_paged;
use coconut_dkg_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use coconut_dkg_common::types::{EpochState, MinimumDepositResponse};
use cosmwasm_std::{
    entry_point, to_binary, Coin, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
};
use cw4::Cw4Contract;
use epoch_state::storage::{advance_epoch_state, CURRENT_EPOCH_STATE};

mod constants;
mod dealers;
mod dealings;
mod epoch_state;
mod error;
mod state;
mod verification_key_shares;

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let multisig_addr = deps.api.addr_validate(&msg.multisig_addr)?;
    ADMIN.set(deps.branch(), Some(multisig_addr.clone()))?;

    let group_addr = Cw4Contract(deps.api.addr_validate(&msg.group_addr).map_err(|_| {
        ContractError::InvalidGroup {
            addr: msg.group_addr.clone(),
        }
    })?);

    let state = State {
        group_addr,
        multisig_addr,
        mix_denom: msg.mix_denom,
    };
    STATE.save(deps.storage, &state)?;

    CURRENT_EPOCH_STATE.save(deps.storage, &EpochState::default())?;

    Ok(Response::default())
}

/// Handle an incoming message
#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::RegisterDealer { bte_key_with_proof } => {
            dealers::transactions::try_add_dealer(deps, info, bte_key_with_proof)
        }
        ExecuteMsg::CommitDealing { dealing_bytes } => {
            dealings::transactions::try_commit_dealings(deps, info, dealing_bytes)
        }
        ExecuteMsg::CommitVerificationKeyShare { share } => {
            verification_key_shares::transactions::try_commit_verification_key_share(
                deps, env, info, share,
            )
        }
        ExecuteMsg::VerifyVerificationKeyShare { owner } => {
            verification_key_shares::transactions::try_verify_verification_key_share(
                deps, info, owner,
            )
        }
        ExecuteMsg::DebugUnsafeResetAll { init_msg } => {
            reset_contract_state(deps, env, info, init_msg)
        }
        ExecuteMsg::AdvanceEpochState {} => advance_epoch_state(deps, info),
    }
}

fn reset_contract_state(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    init_msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    // this resets the epoch
    instantiate(deps.branch(), env, info, init_msg)?;

    // clear all dealings, public keys, etc
    let current = dealers::storage::current_dealers()
        .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<Result<Vec<_>, _>>()?;
    let past = dealers::storage::past_dealers()
        .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<Result<Vec<_>, _>>()?;
    let dealings = dealings::storage::DEALINGS_BYTES[0]
        .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<Result<Vec<_>, _>>()?;

    for dealer in current {
        dealers::storage::current_dealers().remove(deps.storage, &dealer)?;
    }

    for dealer in past {
        dealers::storage::past_dealers().remove(deps.storage, &dealer)?;
    }

    for addr in dealings {
        for map in dealings::storage::DEALINGS_BYTES {
            map.remove(deps.storage, &addr);
        }
    }

    dealers::storage::NODE_INDEX_COUNTER.save(deps.storage, &0u64)?;

    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let response = match msg {
        QueryMsg::GetCurrentEpochState {} => to_binary(&query_current_epoch_state(deps.storage)?)?,
        QueryMsg::GetDealerDetails { dealer_address } => {
            to_binary(&query_dealer_details(deps, dealer_address)?)?
        }
        QueryMsg::GetCurrentDealers { limit, start_after } => {
            to_binary(&query_current_dealers_paged(deps, start_after, limit)?)?
        }
        QueryMsg::GetPastDealers { limit, start_after } => {
            to_binary(&query_past_dealers_paged(deps, start_after, limit)?)?
        }
        QueryMsg::GetDepositAmount {} => to_binary(&MinimumDepositResponse::new(Coin::new(
            MINIMUM_DEPOSIT.u128(),
            STATE.load(deps.storage)?.mix_denom,
        )))?,
        QueryMsg::GetDealing {
            idx,
            limit,
            start_after,
        } => to_binary(&query_dealings_paged(deps, idx, start_after, limit)?)?,
        QueryMsg::GetVerificationKeys { limit, start_after } => {
            to_binary(&query_vk_shares_paged(deps, start_after, limit)?)?
        }
    };

    Ok(response)
}

#[entry_point]
pub fn migrate(_deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let msg = InstantiateMsg {
            group_addr: "group_addr".to_string(),
            multisig_addr: "multisig_addr".to_string(),
            mix_denom: "nym".to_string(),
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info, msg);
        assert!(res.is_ok())
    }
}
