// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::queries::{query_current_dealers_paged, query_past_dealers_paged};
use crate::epoch::queries::query_current_epoch;
use crate::error::ContractError;
use coconut_dkg_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use coconut_dkg_common::types::{Epoch, EpochState};
use cosmwasm_std::{
    entry_point, to_binary, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
};
use epoch::storage as epoch_storage;

mod constants;
mod dealers;
mod epoch;
mod error;
mod support;

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if msg.public_key_submission_end_height < env.block.height {
        return Err(ContractError::EpochStateFinishInPast);
    }

    epoch_storage::CURRENT_EPOCH.save(
        deps.storage,
        &Epoch {
            id: 0,
            state: EpochState::PublicKeySubmission {
                begun_at: env.block.height,
                finish_by: msg.public_key_submission_end_height,
            },
        },
    )?;
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
        ExecuteMsg::RegisterDealer {
            ed25519_key,
            bte_key_with_proof,
            owner_signature,
            host,
        } => dealers::transactions::try_add_dealer(
            deps,
            env,
            info,
            ed25519_key,
            bte_key_with_proof,
            owner_signature,
            host,
        ),
        ExecuteMsg::CommitDealing {
            epoch_id,
            dealing_digest,
            receivers,
        } => dealers::transactions::try_commit_dealing(
            deps,
            env,
            info,
            epoch_id,
            dealing_digest,
            receivers,
        ),
        ExecuteMsg::UnsafeResetAll { init_msg } => reset_contract_state(deps, env, info, init_msg),
    }
}

fn reset_contract_state(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    init_msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // this resets the epoch
    instantiate(deps.branch(), env, info, init_msg)?;

    // clear all dealings, public keys, etc
    let current = dealers::storage::current_dealers()
        .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<Result<Vec<_>, _>>()?;
    let past = dealers::storage::past_dealers()
        .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<Result<Vec<_>, _>>()?;
    let blacklisted = crate::dealers::storage::BLACKLISTED_DEALERS
        .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<Result<Vec<_>, _>>()?;

    for dealer in current {
        dealers::storage::current_dealers().remove(deps.storage, &dealer)?;
    }

    for dealer in past {
        dealers::storage::past_dealers().remove(deps.storage, &dealer)?;
    }

    for dealer in blacklisted {
        dealers::storage::BLACKLISTED_DEALERS.remove(deps.storage, &dealer);
    }

    crate::dealers::storage::NODE_INDEX_COUNTER.save(deps.storage, &0u64)?;

    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let response = match msg {
        QueryMsg::GetCurrentEpoch {} => to_binary(&query_current_epoch(deps.storage)?)?,
        QueryMsg::GetCurrentDealers { limit, start_after } => {
            to_binary(&query_current_dealers_paged(deps, start_after, limit)?)?
        }
        QueryMsg::GetPastDealers { limit, start_after } => {
            to_binary(&query_past_dealers_paged(deps, start_after, limit)?)?
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
    use config::defaults::DENOM;
    use cosmwasm_std::coins;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let msg = InstantiateMsg {
            public_key_submission_end_height: env.block.height + 123,
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info, msg);
        assert!(res.is_ok())
    }
}
