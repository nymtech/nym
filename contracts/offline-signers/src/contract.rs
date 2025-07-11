// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::queries::{
    query_active_proposal, query_active_proposals_paged, query_admin, query_config,
    query_current_signing_status, query_last_status_reset, query_last_status_reset_paged,
    query_offline_signer_information, query_offline_signers_addresses_at_height,
    query_offline_signers_paged, query_proposal, query_proposals_paged,
    query_signing_status_at_height, query_vote_information, query_votes_paged,
};
use crate::storage::NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE;
use crate::transactions::{
    try_propose_or_vote, try_reset_offline_status, try_update_contract_admin,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use nym_contracts_common::set_build_information;
use nym_offline_signers_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NymOfflineSignersContractError, QueryMsg,
};

const CONTRACT_NAME: &str = "crate:nym-offline-signers-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, NymOfflineSignersContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    let dkg_contract_address = deps.api.addr_validate(&msg.dkg_contract_address)?;

    NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE.initialise(
        deps,
        env,
        info.sender,
        dkg_contract_address,
        msg.config,
    )?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, NymOfflineSignersContractError> {
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => try_update_contract_admin(deps, info, admin),
        ExecuteMsg::ProposeOrVote { signer } => try_propose_or_vote(deps, env, info, signer),
        ExecuteMsg::ResetOfflineStatus {} => try_reset_offline_status(deps, env, info),
    }
}

#[entry_point]
pub fn query(
    deps: Deps,
    env: Env,
    msg: QueryMsg,
) -> Result<Binary, NymOfflineSignersContractError> {
    match msg {
        QueryMsg::Admin {} => Ok(to_json_binary(&query_admin(deps)?)?),
        QueryMsg::GetConfig {} => Ok(to_json_binary(&query_config(deps)?)?),
        QueryMsg::GetActiveProposal { signer } => {
            Ok(to_json_binary(&query_active_proposal(deps, env, signer)?)?)
        }
        QueryMsg::GetProposal { proposal_id } => {
            Ok(to_json_binary(&query_proposal(deps, env, proposal_id)?)?)
        }
        QueryMsg::GetVoteInformation { voter, proposal } => Ok(to_json_binary(
            &query_vote_information(deps, voter, proposal)?,
        )?),
        QueryMsg::GetOfflineSignerInformation { signer } => Ok(to_json_binary(
            &query_offline_signer_information(deps, signer)?,
        )?),
        QueryMsg::GetOfflineSignersAddressesAtHeight { height } => Ok(to_json_binary(
            &query_offline_signers_addresses_at_height(deps, height)?,
        )?),
        QueryMsg::GetLastStatusReset { signer } => {
            Ok(to_json_binary(&query_last_status_reset(deps, signer)?)?)
        }
        QueryMsg::GetActiveProposalsPaged { start_after, limit } => Ok(to_json_binary(
            &query_active_proposals_paged(deps, env, start_after, limit)?,
        )?),
        QueryMsg::GetProposalsPaged { start_after, limit } => Ok(to_json_binary(
            &query_proposals_paged(deps, start_after, limit)?,
        )?),
        QueryMsg::GetVotesPaged {
            proposal,
            start_after,
            limit,
        } => Ok(to_json_binary(&query_votes_paged(
            deps,
            proposal,
            start_after,
            limit,
        )?)?),
        QueryMsg::GetOfflineSignersPaged { start_after, limit } => Ok(to_json_binary(
            &query_offline_signers_paged(deps, start_after, limit)?,
        )?),
        QueryMsg::GetLastStatusResetPaged { start_after, limit } => Ok(to_json_binary(
            &query_last_status_reset_paged(deps, start_after, limit)?,
        )?),
        QueryMsg::CurrentSigningStatus {} => {
            Ok(to_json_binary(&query_current_signing_status(deps)?)?)
        }
        QueryMsg::SigningStatusAtHeight { block_height } => Ok(to_json_binary(
            &query_signing_status_at_height(deps, block_height)?,
        )?),
    }
}

#[entry_point]
pub fn migrate(
    deps: DepsMut,
    _: Env,
    _msg: MigrateMsg,
) -> Result<Response, NymOfflineSignersContractError> {
    set_build_information!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod contract_instantiation {
        use super::*;
        use crate::storage::NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE;
        use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};

        #[test]
        fn sets_contract_admin_to_the_message_sender() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let some_sender = deps.api.addr_make("some_sender");
            let dummy_dkg_contract = deps.api.addr_make("dkg_contract");

            instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                InstantiateMsg {
                    dkg_contract_address: dummy_dkg_contract.to_string(),
                    config: Default::default(),
                },
            )?;

            NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
                .contract_admin
                .assert_admin(deps.as_ref(), &some_sender)?;

            Ok(())
        }
    }
}
