// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::basic_signing_status;
use crate::storage::{retrieval_limits, NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE};
use cosmwasm_std::{Deps, Env, Order, StdResult};
use cw_controllers::AdminResponse;
use cw_storage_plus::Bound;
use nym_offline_signers_contract_common::{
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
    use crate::testing::{
        init_contract_tester, init_custom_contract_tester, OfflineSignersContractTesterExt,
    };
    use cosmwasm_std::Decimal;
    use nym_contracts_common_testing::{ChainOpts, ContractOpts};
    use nym_offline_signers_contract_common::{
        InstantiateMsg, OfflineSignerInformation, ProposalWithResolution, StatusResetInformation,
        VoteInformation,
    };

    #[cfg(test)]
    mod admin_query {
        use super::*;
        use crate::testing::init_contract_tester;
        use nym_contracts_common_testing::{AdminExt, ChainOpts, ContractOpts, RandExt};
        use nym_offline_signers_contract_common::ExecuteMsg;

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

    #[test]
    #[allow(clippy::panic)]
    fn active_proposal_query() -> anyhow::Result<()> {
        let mut tester = init_contract_tester();

        let signer = tester.random_group_member();

        // invalid address
        let res = query_active_proposal(tester.deps(), tester.env(), "bad-address".to_string());
        assert!(res.is_err());

        // new signer - no active proposals
        let res = query_active_proposal(tester.deps(), tester.env(), signer.to_string())?;
        assert!(res.proposal.is_none());

        // initial proposal
        let id1 = tester.make_proposal(&signer);

        let Some(proposal) =
            query_active_proposal(tester.deps(), tester.env(), signer.to_string())?.proposal
        else {
            panic!("test failure - no proposal")
        };
        assert_eq!(id1, proposal.proposal.id);
        assert_eq!(signer.clone(), proposal.proposal.proposed_offline_signer);

        assert!(!proposal.passed);
        assert!(!proposal.voting_finished);

        // passed
        tester.add_votes(id1);
        let Some(proposal) =
            query_active_proposal(tester.deps(), tester.env(), signer.to_string())?.proposal
        else {
            panic!("test failure - no proposal")
        };
        assert_eq!(id1, proposal.proposal.id);
        assert_eq!(signer.clone(), proposal.proposal.proposed_offline_signer);

        assert!(proposal.passed);
        assert!(!proposal.voting_finished);

        // voting passed
        tester.advance_day_of_blocks();
        tester.add_votes(id1);
        let Some(proposal) =
            query_active_proposal(tester.deps(), tester.env(), signer.to_string())?.proposal
        else {
            panic!("test failure - no proposal")
        };
        assert_eq!(id1, proposal.proposal.id);
        assert_eq!(signer.clone(), proposal.proposal.proposed_offline_signer);

        assert!(proposal.passed);
        assert!(proposal.voting_finished);

        // marked online - no proposals again
        tester.reset_offline_status(&signer);
        let res = query_active_proposal(tester.deps(), tester.env(), signer.to_string())?;
        assert!(res.proposal.is_none());
        tester.advance_day_of_blocks();

        let id2 = tester.make_proposal(&signer);
        let Some(proposal) =
            query_active_proposal(tester.deps(), tester.env(), signer.to_string())?.proposal
        else {
            panic!("test failure - no proposal")
        };
        assert_ne!(id1, id2);
        assert_eq!(id2, proposal.proposal.id);
        assert_eq!(signer.clone(), proposal.proposal.proposed_offline_signer);

        assert!(!proposal.passed);
        assert!(!proposal.voting_finished);

        tester.advance_day_of_blocks();

        let Some(proposal) =
            query_active_proposal(tester.deps(), tester.env(), signer.to_string())?.proposal
        else {
            panic!("test failure - no proposal")
        };
        assert_eq!(id2, proposal.proposal.id);
        assert_eq!(signer.clone(), proposal.proposal.proposed_offline_signer);

        assert!(!proposal.passed);
        assert!(proposal.voting_finished);

        Ok(())
    }

    #[test]
    #[allow(clippy::panic)]
    fn proposal_query() -> anyhow::Result<()> {
        let mut tester = init_contract_tester();

        let signer = tester.random_group_member();

        // new signer - no active proposals
        let res = query_proposal(tester.deps(), tester.env(), 1)?;
        assert!(res.proposal.is_none());

        // initial proposal
        let id1 = tester.make_proposal(&signer);
        // sanity check
        assert_eq!(id1, 1);

        let Some(proposal) = query_proposal(tester.deps(), tester.env(), 1)?.proposal else {
            panic!("test failure - no proposal")
        };
        assert_eq!(id1, proposal.proposal.id);
        assert_eq!(signer.clone(), proposal.proposal.proposed_offline_signer);

        assert!(!proposal.passed);
        assert!(!proposal.voting_finished);

        // passed
        tester.add_votes(id1);
        let Some(proposal) = query_proposal(tester.deps(), tester.env(), 1)?.proposal else {
            panic!("test failure - no proposal")
        };
        assert_eq!(id1, proposal.proposal.id);
        assert_eq!(signer.clone(), proposal.proposal.proposed_offline_signer);

        assert!(proposal.passed);
        assert!(!proposal.voting_finished);

        // voting passed
        tester.advance_day_of_blocks();
        tester.add_votes(id1);
        let Some(proposal) = query_proposal(tester.deps(), tester.env(), 1)?.proposal else {
            panic!("test failure - no proposal")
        };
        assert_eq!(id1, proposal.proposal.id);
        assert_eq!(signer.clone(), proposal.proposal.proposed_offline_signer);

        assert!(proposal.passed);
        assert!(proposal.voting_finished);

        // marked online - proposals still exists!
        tester.reset_offline_status(&signer);
        let Some(proposal) = query_proposal(tester.deps(), tester.env(), 1)?.proposal else {
            panic!("test failure - no proposal")
        };
        assert_eq!(id1, proposal.proposal.id);
        assert_eq!(signer.clone(), proposal.proposal.proposed_offline_signer);

        assert!(proposal.passed);
        assert!(proposal.voting_finished);

        tester.advance_day_of_blocks();

        let id2 = tester.make_proposal(&signer);
        let Some(proposal) = query_proposal(tester.deps(), tester.env(), 2)?.proposal else {
            panic!("test failure - no proposal")
        };
        assert_eq!(id2, 2);
        assert_eq!(id2, proposal.proposal.id);
        assert_eq!(signer.clone(), proposal.proposal.proposed_offline_signer);

        assert!(!proposal.passed);
        assert!(!proposal.voting_finished);

        tester.advance_day_of_blocks();

        let Some(proposal) = query_proposal(tester.deps(), tester.env(), 2)?.proposal else {
            panic!("test failure - no proposal")
        };
        assert_eq!(id2, proposal.proposal.id);
        assert_eq!(signer.clone(), proposal.proposal.proposed_offline_signer);

        assert!(!proposal.passed);
        assert!(proposal.voting_finished);

        Ok(())
    }

    #[test]
    fn vote_information_query() -> anyhow::Result<()> {
        let mut tester = init_contract_tester();
        let signer = tester.random_group_member();
        let voter1 = tester.random_group_member();
        let voter2 = tester.random_group_member();

        let proposal_id = tester.insert_empty_proposal(&signer);
        let res = query_vote_information(tester.deps(), "bad-address".to_string(), proposal_id);
        assert!(res.is_err());

        let res1 = query_vote_information(tester.deps(), voter1.to_string(), proposal_id)?;
        let res2 = query_vote_information(tester.deps(), voter2.to_string(), proposal_id)?;

        assert!(res1.vote.is_none());
        assert!(res2.vote.is_none());

        tester.add_vote(proposal_id, &voter1);
        let res1 = query_vote_information(tester.deps(), voter1.to_string(), proposal_id)?;
        let res2 = query_vote_information(tester.deps(), voter2.to_string(), proposal_id)?;

        assert_eq!(
            res1.vote.unwrap(),
            VoteInformation {
                voted_at: tester.env().block,
            }
        );
        assert!(res2.vote.is_none());

        tester.next_block();
        tester.add_vote(proposal_id, &voter2);
        let res1 = query_vote_information(tester.deps(), voter1.to_string(), proposal_id)?;
        let res2 = query_vote_information(tester.deps(), voter2.to_string(), proposal_id)?;
        assert_ne!(res1, res2);
        assert_eq!(
            res2.vote.unwrap(),
            VoteInformation {
                voted_at: tester.env().block,
            }
        );

        Ok(())
    }

    #[test]
    fn offline_signer_information_query() -> anyhow::Result<()> {
        let mut tester = init_contract_tester();
        let signer = tester.random_group_member();

        assert!(
            query_offline_signer_information(tester.deps(), "bad-address".to_string()).is_err()
        );
        assert!(
            query_offline_signer_information(tester.deps(), signer.to_string())?
                .information
                .is_none()
        );

        tester.insert_offline_signer(&signer);
        let res = query_offline_signer_information(tester.deps(), signer.to_string())?
            .information
            .unwrap();
        assert_eq!(
            res,
            OfflineSignerInformation {
                marked_offline_at: tester.env().block,
                associated_proposal: 1,
            }
        );

        Ok(())
    }

    #[cfg(test)]
    mod offline_signers_at_height {
        use super::*;

        #[test]
        fn current_height() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let signer1 = tester.random_group_member();
            let signer2 = tester.random_group_member();
            let signer3 = tester.random_group_member();

            assert!(
                query_offline_signers_addresses_at_height(tester.deps(), None)?
                    .addresses
                    .is_empty()
            );

            tester.insert_offline_signer(&signer1);
            assert_eq!(
                query_offline_signers_addresses_at_height(tester.deps(), None)?.addresses,
                vec![signer1.clone()]
            );
            tester.insert_offline_signer(&signer2);
            assert_eq!(
                query_offline_signers_addresses_at_height(tester.deps(), None)?.addresses,
                vec![signer1.clone(), signer2.clone()]
            );
            tester.insert_offline_signer(&signer3);
            assert_eq!(
                query_offline_signers_addresses_at_height(tester.deps(), None)?.addresses,
                vec![signer1.clone(), signer2.clone(), signer3.clone()]
            );

            tester.advance_day_of_blocks();
            tester.reset_offline_status(&signer2);
            assert_eq!(
                query_offline_signers_addresses_at_height(tester.deps(), None)?.addresses,
                vec![signer1.clone(), signer3]
            );
            Ok(())
        }

        #[test]
        fn specific_height() -> anyhow::Result<()> {
            let mut tester = init_contract_tester();
            let signer1 = tester.random_group_member();
            let signer2 = tester.random_group_member();
            let signer3 = tester.random_group_member();

            let h1 = tester.env().block.height;
            assert!(
                query_offline_signers_addresses_at_height(tester.deps(), None)?
                    .addresses
                    .is_empty()
            );

            tester.next_block();
            tester.insert_offline_signer(&signer1);
            let h2 = tester.env().block.height;

            tester.next_block();
            tester.insert_offline_signer(&signer2);
            let h3 = tester.env().block.height;

            tester.next_block();
            tester.insert_offline_signer(&signer3);
            let h4 = tester.env().block.height;

            tester.advance_day_of_blocks();
            tester.reset_offline_status(&signer2);
            let h5 = tester.env().block.height;

            assert!(
                query_offline_signers_addresses_at_height(tester.deps(), Some(h1 + 1))?
                    .addresses
                    .is_empty()
            );
            assert_eq!(
                query_offline_signers_addresses_at_height(tester.deps(), Some(h2 + 1))?.addresses,
                vec![signer1.clone()]
            );
            assert_eq!(
                query_offline_signers_addresses_at_height(tester.deps(), Some(h3 + 1))?.addresses,
                vec![signer1.clone(), signer2.clone()]
            );
            assert_eq!(
                query_offline_signers_addresses_at_height(tester.deps(), Some(h4 + 1))?.addresses,
                vec![signer1.clone(), signer2.clone(), signer3.clone()]
            );
            assert_eq!(
                query_offline_signers_addresses_at_height(tester.deps(), Some(h5 + 1))?.addresses,
                vec![signer1.clone(), signer3]
            );
            Ok(())
        }
    }

    #[test]
    fn last_status_reset_query() -> anyhow::Result<()> {
        let mut tester = init_contract_tester();
        let signer = tester.random_group_member();

        assert!(query_last_status_reset(tester.deps(), "bad-address".to_string()).is_err());
        assert!(query_last_status_reset(tester.deps(), signer.to_string())?
            .information
            .is_none());

        tester.insert_offline_signer(&signer);
        assert!(query_last_status_reset(tester.deps(), signer.to_string())?
            .information
            .is_none());
        tester.advance_day_of_blocks();
        tester.reset_offline_status(&signer);

        let res1 = query_last_status_reset(tester.deps(), signer.to_string())?
            .information
            .unwrap();
        assert_eq!(
            res1,
            StatusResetInformation {
                status_reset_at: tester.env().block,
            }
        );

        tester.advance_day_of_blocks();
        tester.insert_offline_signer(&signer);
        let res2 = query_last_status_reset(tester.deps(), signer.to_string())?
            .information
            .unwrap();
        assert_eq!(res1, res2);
        tester.advance_day_of_blocks();
        tester.reset_offline_status(&signer);

        let res3 = query_last_status_reset(tester.deps(), signer.to_string())?
            .information
            .unwrap();
        assert_eq!(
            res3,
            StatusResetInformation {
                status_reset_at: tester.env().block,
            }
        );

        Ok(())
    }

    #[test]
    fn active_proposals_paged_query() -> anyhow::Result<()> {
        let mut tester = init_custom_contract_tester(
            10,
            InstantiateMsg {
                dkg_contract_address: "".to_string(),
                config: Config {
                    required_quorum: Decimal::percent(20),
                    ..Default::default()
                },
            },
        );

        let signer1 = tester.random_group_member();
        let signer2 = tester.random_group_member();
        let signer3 = tester.random_group_member();
        let signer4 = tester.random_group_member();

        // expired
        let id1 = tester.insert_empty_proposal(&signer1);
        tester.advance_day_of_blocks();

        // passed
        let id2 = tester.insert_empty_proposal(&signer2);

        // not passed
        let id3 = tester.insert_empty_proposal(&signer3);

        // not voted on
        let id4 = tester.insert_empty_proposal(&signer4);

        let mut signers_with_proposals = [
            (signer1, id1),
            (signer2, id2),
            (signer3, id3),
            (signer4, id4),
        ];
        signers_with_proposals.sort_by_key(|a| a.0.clone());

        let voter1 = tester.random_group_member();
        let voter2 = tester.random_group_member();

        tester.add_vote(id2, &voter1);
        tester.add_vote(id2, &voter2);

        tester.add_vote(id3, &voter1);

        let active = query_active_proposals_paged(tester.deps(), tester.env(), None, None)?;

        let prop1 = tester.load_proposal(signers_with_proposals[0].1).unwrap();
        let prop2 = tester.load_proposal(signers_with_proposals[1].1).unwrap();
        let prop3 = tester.load_proposal(signers_with_proposals[2].1).unwrap();
        let prop4 = tester.load_proposal(signers_with_proposals[3].1).unwrap();

        assert_eq!(
            active.active_proposals,
            vec![
                ProposalWithResolution {
                    proposal: prop1,
                    passed: false,
                    voting_finished: true,
                },
                ProposalWithResolution {
                    proposal: prop2,
                    passed: false,
                    voting_finished: false,
                },
                ProposalWithResolution {
                    proposal: prop3,
                    passed: false,
                    voting_finished: false,
                },
                ProposalWithResolution {
                    proposal: prop4,
                    passed: true,
                    voting_finished: false,
                }
            ]
        );

        let res = query_active_proposals_paged(tester.deps(), tester.env(), None, Some(0))?;
        assert!(res.active_proposals.is_empty());

        let res = query_active_proposals_paged(
            tester.deps(),
            tester.env(),
            Some(signers_with_proposals[3].0.to_string()),
            None,
        )?;
        assert!(res.active_proposals.is_empty());

        Ok(())
    }

    #[test]
    fn proposals_paged_query() -> anyhow::Result<()> {
        let mut tester = init_contract_tester();

        let signer1 = tester.random_group_member();
        let signer2 = tester.random_group_member();
        let signer3 = tester.random_group_member();
        let signer4 = tester.random_group_member();

        let id1 = tester.insert_empty_proposal(&signer1);
        let id2 = tester.insert_empty_proposal(&signer2);
        let id3 = tester.insert_empty_proposal(&signer3);
        let id4 = tester.insert_empty_proposal(&signer4);

        let active = query_proposals_paged(tester.deps(), None, None)?;

        let prop1 = tester.load_proposal(id1).unwrap();
        let prop2 = tester.load_proposal(id2).unwrap();
        let prop3 = tester.load_proposal(id3).unwrap();
        let prop4 = tester.load_proposal(id4).unwrap();

        assert_eq!(active.proposals, vec![prop1, prop2, prop3, prop4,]);

        let res = query_proposals_paged(tester.deps(), None, Some(0))?;
        assert!(res.proposals.is_empty());

        let res = query_proposals_paged(tester.deps(), Some(id4), None)?;
        assert!(res.proposals.is_empty());

        Ok(())
    }

    #[test]
    fn votes_paged_query() -> anyhow::Result<()> {
        let mut tester = init_contract_tester();

        let signer1 = tester.random_group_member();
        let signer2 = tester.random_group_member();
        let signer3 = tester.random_group_member();

        let id1 = tester.insert_empty_proposal(&signer1);
        let id2 = tester.insert_empty_proposal(&signer2);
        let id3 = tester.insert_empty_proposal(&signer3);

        let voter1 = tester.random_group_member();
        let voter2 = tester.random_group_member();

        tester.add_vote(id2, &voter1);
        tester.add_vote(id2, &voter2);

        tester.add_vote(id3, &voter1);

        let votes1 = query_votes_paged(tester.deps(), id1, None, None)?;
        let votes2 = query_votes_paged(tester.deps(), id2, None, None)?;
        let votes3 = query_votes_paged(tester.deps(), id3, None, None)?;

        assert!(votes1.votes.is_empty());
        assert_eq!(
            votes2.votes,
            vec![
                VoteDetails {
                    voter: voter1.clone(),
                    information: VoteInformation {
                        voted_at: tester.env().block,
                    },
                },
                VoteDetails {
                    voter: voter2.clone(),
                    information: VoteInformation {
                        voted_at: tester.env().block,
                    },
                }
            ]
        );

        assert_eq!(
            votes3.votes,
            vec![VoteDetails {
                voter: voter1.clone(),
                information: VoteInformation {
                    voted_at: tester.env().block
                },
            }]
        );

        let res = query_votes_paged(tester.deps(), 2, None, Some(0))?;
        assert!(res.votes.is_empty());

        let res = query_votes_paged(tester.deps(), id3, Some(voter1.to_string()), None)?;
        assert!(res.votes.is_empty());

        Ok(())
    }

    #[test]
    fn offline_signers_paged_query() -> anyhow::Result<()> {
        let mut tester = init_contract_tester();

        let mut signers = [
            tester.random_group_member(),
            tester.random_group_member(),
            tester.random_group_member(),
        ];
        signers.sort_unstable();

        tester.insert_offline_signer(&signers[0]);
        tester.insert_offline_signer(&signers[1]);
        tester.insert_offline_signer(&signers[2]);

        let res = query_offline_signers_paged(tester.deps(), None, None)?;
        assert_eq!(
            res.offline_signers,
            vec![
                OfflineSignerDetails {
                    information: OfflineSignerInformation {
                        marked_offline_at: tester.env().block,
                        associated_proposal: 1
                    },
                    signer: signers[0].clone()
                },
                OfflineSignerDetails {
                    information: OfflineSignerInformation {
                        marked_offline_at: tester.env().block,
                        associated_proposal: 2
                    },
                    signer: signers[1].clone()
                },
                OfflineSignerDetails {
                    information: OfflineSignerInformation {
                        marked_offline_at: tester.env().block,
                        associated_proposal: 3
                    },
                    signer: signers[2].clone()
                }
            ]
        );

        let res = query_offline_signers_paged(tester.deps(), None, Some(0))?;
        assert!(res.offline_signers.is_empty());

        let res = query_offline_signers_paged(tester.deps(), Some(signers[2].to_string()), None)?;
        assert!(res.offline_signers.is_empty());

        Ok(())
    }

    #[test]
    fn last_status_reset_paged_query() -> anyhow::Result<()> {
        let mut tester = init_contract_tester();

        let mut signers = [
            tester.random_group_member(),
            tester.random_group_member(),
            tester.random_group_member(),
        ];
        signers.sort_unstable();

        tester.insert_offline_signer(&signers[0]);
        tester.insert_offline_signer(&signers[1]);
        tester.insert_offline_signer(&signers[2]);

        tester.advance_day_of_blocks();
        tester.reset_offline_status(&signers[0]);
        tester.reset_offline_status(&signers[1]);
        tester.reset_offline_status(&signers[2]);

        let res = query_last_status_reset_paged(tester.deps(), None, None)?;
        assert_eq!(
            res.status_resets,
            vec![
                LastStatusResetDetails {
                    information: StatusResetInformation {
                        status_reset_at: tester.env().block
                    },
                    signer: signers[0].clone()
                },
                LastStatusResetDetails {
                    information: StatusResetInformation {
                        status_reset_at: tester.env().block
                    },
                    signer: signers[1].clone()
                },
                LastStatusResetDetails {
                    information: StatusResetInformation {
                        status_reset_at: tester.env().block
                    },
                    signer: signers[2].clone()
                }
            ]
        );

        let res = query_last_status_reset_paged(tester.deps(), None, Some(0))?;
        assert!(res.status_resets.is_empty());

        let res = query_last_status_reset_paged(tester.deps(), Some(signers[2].to_string()), None)?;
        assert!(res.status_resets.is_empty());

        Ok(())
    }
}
