// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::{group_members, DkgContractQuerier};
use crate::storage::{retrieval_limits, NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE};
use cosmwasm_std::{Deps, Env, Order, StdResult};
use cw4::Cw4Contract;
use cw_controllers::AdminResponse;
use cw_storage_plus::Bound;
use nym_offline_signers_common::{
    ActiveProposalResponse, ActiveProposalsPagedResponse, Config, LastStatusResetDetails,
    LastStatusResetPagedResponse, LastStatusResetResponse, NymOfflineSignersContractError,
    OfflineSignerDetails, OfflineSignerResponse, OfflineSignersPagedResponse, ProposalId,
    ProposalResponse, ProposalWithResolution, ProposalsPagedResponse, SigningStatusResponse,
    VoteDetails, VoteResponse, VotesPagedResponse,
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
        .may_load(deps.storage, &signer)?;

    Ok(OfflineSignerResponse { information })
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

pub fn query_signing_status(
    deps: Deps,
) -> Result<SigningStatusResponse, NymOfflineSignersContractError> {
    let dkg_contract_address = NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .dkg_contract
        .load(deps.storage)?;

    let dkg_epoch = deps.querier.query_dkg_epoch(&dkg_contract_address)?;

    // if DKG exchange is currently in progress, retrieve dealers and threshold from the PREVIOUS epoch
    // as that'd be the set used for issuing credentials
    let epoch_id = if dkg_epoch.state.is_final() {
        dkg_epoch.epoch_id
    } else {
        dkg_epoch.epoch_id.saturating_sub(1)
    };

    let dkg_threshold = deps
        .querier
        .query_dkg_threshold(&dkg_contract_address, epoch_id)?;

    let group_contract = Cw4Contract::new(
        deps.querier
            .query_dkg_cw4_contract_address(&dkg_contract_address)?,
    );
    let total_group_members = group_members(&deps.querier, &group_contract)?;

    let dkg_dealers = deps
        .querier
        .query_dkg_dealers(&dkg_contract_address, epoch_id)?;

    // we need to filter out signers marked as offline that are not part of the corresponding dkg epoch
    let offline_signers = NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .offline_signers
        .keys(deps.storage, None, None, Order::Ascending)
        // given the list won't contain more than a dozen or so entries, linear lookup is faster
        // than trying to save it in a set
        .map(|offline_signer| offline_signer.map(|s| dkg_dealers.contains(&s)))
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .filter(|is_dealer| *is_dealer)
        .count() as u32;

    let available_signers = (dkg_dealers.len() as u32).saturating_sub(offline_signers);

    Ok(SigningStatusResponse {
        dkg_epoch_id: epoch_id,
        signing_threshold: dkg_threshold,
        total_group_members,
        current_registered_dealers: dkg_dealers.len() as u32,
        offline_signers,
        threshold_available: available_signers as u64 >= dkg_threshold,
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
