// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::basic_signing_status;
use crate::storage::{retrieval_limits, NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE};
use cosmwasm_std::{Deps, Env, Order, StdResult};
use cw_controllers::AdminResponse;
use cw_storage_plus::Bound;
use nym_offline_signers_common::{
    ActiveProposalResponse, ActiveProposalsPagedResponse, Config, LastStatusResetDetails,
    LastStatusResetPagedResponse, LastStatusResetResponse, NymOfflineSignersContractError,
    OfflineSignerDetails, OfflineSignerResponse, OfflineSignersAddressesResponse,
    OfflineSignersPagedResponse, ProposalId, ProposalResponse, ProposalsPagedResponse,
    SigningStatusAtHeightResponse, SigningStatusResponse, VoteDetails, VoteResponse,
    VotesPagedResponse,
};

pub fn query_admin(deps: Deps) -> Result<AdminResponse, NymOfflineSignersContractError> {
    NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .contract_admin
        .query_admin(deps)
        .map_err(Into::into)
}

pub fn query_config(deps: Deps) -> Result<Config, NymOfflineSignersContractError> {
    NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .config
        .load(deps.storage)
        .map_err(Into::into)
}

pub fn query_active_proposal(
    deps: Deps,
    env: Env,
    signer: String,
) -> Result<ActiveProposalResponse, NymOfflineSignersContractError> {
    let signer = deps.api.addr_validate(&signer)?;

    let Some(proposal) =
        NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE.try_load_active_proposal(deps.storage, &signer)?
    else {
        return Ok(ActiveProposalResponse { proposal: None });
    };

    Ok(ActiveProposalResponse {
        proposal: Some(
            NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE.add_proposal_resolution(deps, &env, proposal)?,
        ),
    })
}

pub fn query_proposal(
    deps: Deps,
    env: Env,
    proposal_id: ProposalId,
) -> Result<ProposalResponse, NymOfflineSignersContractError> {
    let Some(proposal) = NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .proposals
        .may_load(deps.storage, proposal_id)?
    else {
        return Ok(ProposalResponse { proposal: None });
    };

    Ok(ProposalResponse {
        proposal: Some(
            NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE.add_proposal_resolution(deps, &env, proposal)?,
        ),
    })
}

pub fn query_vote_information(
    deps: Deps,
    voter: String,
    proposal_id: ProposalId,
) -> Result<VoteResponse, NymOfflineSignersContractError> {
    let voter = deps.api.addr_validate(&voter)?;

    let vote = NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .votes
        .may_load(deps.storage, (proposal_id, &voter))?;
    Ok(VoteResponse { vote })
}

pub fn query_offline_signer_information(
    deps: Deps,
    signer: String,
) -> Result<OfflineSignerResponse, NymOfflineSignersContractError> {
    let signer = deps.api.addr_validate(&signer)?;

    let information = NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .offline_signers
        .information
        .may_load(deps.storage, &signer)?;

    Ok(OfflineSignerResponse { information })
}

pub fn query_offline_signers_addresses_at_height(
    deps: Deps,
    height: Option<u64>,
) -> Result<OfflineSignersAddressesResponse, NymOfflineSignersContractError> {
    let addresses = match height {
        Some(height) => NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
            .offline_signers
            .addresses
            .may_load_at_height(deps.storage, height)?
            .unwrap_or_default(),
        None => NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
            .offline_signers
            .addresses
            .load(deps.storage)?,
    };

    Ok(OfflineSignersAddressesResponse { addresses })
}

pub fn query_last_status_reset(
    deps: Deps,
    signer: String,
) -> Result<LastStatusResetResponse, NymOfflineSignersContractError> {
    let signer = deps.api.addr_validate(&signer)?;

    let information = NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .last_status_reset
        .may_load(deps.storage, &signer)?;

    Ok(LastStatusResetResponse { information })
}

pub fn query_active_proposals_paged(
    deps: Deps,
    env: Env,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<ActiveProposalsPagedResponse, NymOfflineSignersContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::ACTIVE_PROPOSALS_DEFAULT_LIMIT)
        .min(retrieval_limits::ACTIVE_PROPOSALS_MAX_LIMIT) as usize;

    let signer = start_after
        .map(|signer| deps.api.addr_validate(&signer))
        .transpose()?;

    let start = signer.as_ref().map(Bound::exclusive);
    let active_proposals = NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .active_proposals
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map_err(Into::into)
                .and_then(|(_, proposal_id)| {
                    NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
                        .proposals
                        .load(deps.storage, proposal_id)
                        .map_err(Into::into)
                })
                .and_then(|proposal| {
                    NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
                        .add_proposal_resolution(deps, &env, proposal)
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let start_next_after = active_proposals
        .last()
        .map(|p| p.proposal.proposed_offline_signer.to_string());

    Ok(ActiveProposalsPagedResponse {
        start_next_after,
        active_proposals,
    })
}

pub fn query_proposals_paged(
    deps: Deps,
    start_after: Option<ProposalId>,
    limit: Option<u32>,
) -> Result<ProposalsPagedResponse, NymOfflineSignersContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::PROPOSALS_DEFAULT_LIMIT)
        .min(retrieval_limits::PROPOSALS_MAX_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let proposals = NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .proposals
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(_, proposal)| proposal))
        .collect::<Result<Vec<_>, _>>()?;

    let start_next_after = proposals.last().map(|p| p.id);

    Ok(ProposalsPagedResponse {
        start_next_after,
        proposals,
    })
}

pub fn query_votes_paged(
    deps: Deps,
    proposal_id: ProposalId,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<VotesPagedResponse, NymOfflineSignersContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::VOTES_DEFAULT_LIMIT)
        .min(retrieval_limits::VOTES_MAX_LIMIT) as usize;

    let voter = start_after
        .map(|voter| deps.api.addr_validate(&voter))
        .transpose()?;
    let start = voter.as_ref().map(Bound::exclusive);

    let votes = NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .votes
        .prefix(proposal_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(voter, information)| VoteDetails { voter, information }))
        .collect::<Result<Vec<_>, _>>()?;

    let start_next_after = votes.last().map(|vote| vote.voter.to_string());

    Ok(VotesPagedResponse {
        start_next_after,
        votes,
    })
}

pub fn query_offline_signers_paged(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<OfflineSignersPagedResponse, NymOfflineSignersContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::OFFLINE_SIGNERS_DEFAULT_LIMIT)
        .min(retrieval_limits::OFFLINE_SIGNERS_MAX_LIMIT) as usize;

    let signer = start_after
        .map(|signer| deps.api.addr_validate(&signer))
        .transpose()?;
    let start = signer.as_ref().map(Bound::exclusive);

    let offline_signers = NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .offline_signers
        .information
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(signer, information)| OfflineSignerDetails {
                information,
                signer,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = offline_signers
        .last()
        .map(|details| details.signer.to_string());

    Ok(OfflineSignersPagedResponse {
        start_next_after,
        offline_signers,
    })
}

pub fn query_last_status_reset_paged(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<LastStatusResetPagedResponse, NymOfflineSignersContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::LAST_STATUS_RESET_DEFAULT_LIMIT)
        .min(retrieval_limits::LAST_STATUS_RESET_MAX_LIMIT) as usize;

    let signer = start_after
        .map(|signer| deps.api.addr_validate(&signer))
        .transpose()?;
    let start = signer.as_ref().map(Bound::exclusive);

    let status_resets = NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .last_status_reset
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(signer, information)| LastStatusResetDetails {
                information,
                signer,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = status_resets
        .last()
        .map(|details| details.signer.to_string());

    Ok(LastStatusResetPagedResponse {
        start_next_after,
        status_resets,
    })
}

pub fn query_current_signing_status(
    deps: Deps,
) -> Result<SigningStatusResponse, NymOfflineSignersContractError> {
    basic_signing_status(deps, None)
}

pub fn query_signing_status_at_height(
    deps: Deps,
    block_height: u64,
) -> Result<SigningStatusAtHeightResponse, NymOfflineSignersContractError> {
    let basic = basic_signing_status(deps, Some(block_height))?;

    Ok(SigningStatusAtHeightResponse {
        block_height,
        dkg_epoch_id: basic.dkg_epoch_id,
        signing_threshold: basic.signing_threshold,
        current_registered_dealers: basic.current_registered_dealers,
        offline_signers: basic.offline_signers,
        threshold_available: basic.threshold_available,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod admin_query {
        use super::*;
        use crate::testing::init_contract_tester;
        use nym_contracts_common_testing::{AdminExt, ChainOpts, ContractOpts, RandExt};
        use nym_offline_signers_common::ExecuteMsg;

        #[test]
        fn returns_current_admin() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let initial_admin = test.admin_unchecked();

            // initial
            let res = query_admin(test.deps())?;
            assert_eq!(res.admin, Some(initial_admin.to_string()));

            let new_admin = test.generate_account();

            // sanity check
            assert_ne!(initial_admin, new_admin);

            // after update
            test.execute_msg(
                initial_admin.clone(),
                &ExecuteMsg::UpdateAdmin {
                    admin: new_admin.to_string(),
                },
            )?;

            let updated_admin = query_admin(test.deps())?;
            assert_eq!(updated_admin.admin, Some(new_admin.to_string()));

            Ok(())
        }
    }
}
