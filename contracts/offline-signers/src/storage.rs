// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::{group_members, DkgContractQuerier};
use cosmwasm_std::{Addr, Decimal, Deps, DepsMut, Env, Order, StdResult, Storage};
use cw4::Cw4Contract;
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map, SnapshotMap, Strategy};
use nym_offline_signers_common::constants::storage_keys;
use nym_offline_signers_common::constants::storage_keys::PROPOSAL_COUNT;
use nym_offline_signers_common::{
    Config, NymOfflineSignersContractError, OfflineSignerInformation, Proposal, ProposalId,
    ProposalWithResolution, StatusResetInformation, VoteInformation,
};

pub const NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE: NymOfflineSignersStorage =
    NymOfflineSignersStorage::new();

pub struct NymOfflineSignersStorage {
    // address of the contract admin
    pub(crate) contract_admin: Admin,

    // address of the associated DKG contract
    pub(crate) dkg_contract: Item<Addr>,

    // configurable (by the admin) values of this contract
    pub(crate) config: Item<Config>,

    // map between given signer and a currently active (if applicable) proposal id
    // note: one signer can have only a single active proposal against them at a given time
    pub(crate) active_proposals: Map<&'static Addr, ProposalId>,

    // all proposals ever created - realistically we'll ever see a handful of them,
    // so leaving them is fine
    pub(crate) proposals: Map<ProposalId, Proposal>,

    // votes information (proposal, voter) => vote
    pub(crate) votes: Map<(ProposalId, &'static Addr), VoteInformation>,

    // map of all signers marked as offline
    pub(crate) offline_signers: SnapshotMap<&'static Addr, OfflineSignerInformation>,

    // holds information on when signers last reset their status after going back online
    // (for full history you'd have to scrape the chain data; the system doesn't need it so it doesn't hold it)
    pub(crate) last_status_reset: Map<&'static Addr, StatusResetInformation>,

    // keep track of the current proposal id counter
    pub(crate) proposal_count: Item<u64>,
}

impl NymOfflineSignersStorage {
    #[allow(clippy::new_without_default)]
    pub(crate) const fn new() -> Self {
        NymOfflineSignersStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            dkg_contract: Item::new(storage_keys::DKG_CONTRACT),
            config: Item::new(storage_keys::CONFIG),
            active_proposals: Map::new(storage_keys::ACTIVE_PROPOSALS),
            proposals: Map::new(storage_keys::PROPOSALS),
            votes: Map::new(storage_keys::VOTES),
            offline_signers: SnapshotMap::new(
                storage_keys::OFFLINE_SIGNERS_PRIMARY,
                storage_keys::OFFLINE_SIGNERS_CHECKPOINTS,
                storage_keys::OFFLINE_SIGNERS_CHANGELOG,
                Strategy::EveryBlock,
            ),
            last_status_reset: Map::new(storage_keys::LAST_STATUS_RESET),
            proposal_count: Item::new(PROPOSAL_COUNT),
        }
    }

    fn next_proposal_id(&self, storage: &mut dyn Storage) -> StdResult<ProposalId> {
        let id: ProposalId = self.proposal_count.may_load(storage)?.unwrap_or_default() + 1;
        self.proposal_count.save(storage, &id)?;
        Ok(id)
    }

    fn insert_new_active_proposal(
        &self,
        storage: &mut dyn Storage,
        env: &Env,
        proposer: &Addr,
        proposed_offline_signer: &Addr,
    ) -> Result<ProposalId, NymOfflineSignersContractError> {
        let id = self.next_proposal_id(storage)?;
        self.proposals.save(
            storage,
            id,
            &Proposal {
                created_at: env.block.clone(),
                id,
                proposed_offline_signer: proposed_offline_signer.clone(),
                proposer: proposer.clone(),
            },
        )?;
        self.active_proposals.save(storage, proposer, &id)?;
        Ok(id)
    }

    fn is_admin(&self, deps: Deps, addr: &Addr) -> Result<bool, NymOfflineSignersContractError> {
        self.contract_admin.is_admin(deps, addr).map_err(Into::into)
    }

    fn ensure_is_admin(
        &self,
        deps: Deps,
        addr: &Addr,
    ) -> Result<(), NymOfflineSignersContractError> {
        self.contract_admin
            .assert_admin(deps, addr)
            .map_err(Into::into)
    }

    pub fn initialise(
        &self,
        mut deps: DepsMut,
        env: Env,
        admin: Addr,
    ) -> Result<(), NymOfflineSignersContractError> {
        let _ = deps;
        let _ = env;

        // set the contract admin
        self.contract_admin
            .set(deps.branch(), Some(admin.clone()))?;
        Ok(())
    }

    pub(crate) fn try_load_active_proposal(
        &self,
        storage: &dyn Storage,
        signer: &Addr,
    ) -> Result<Option<Proposal>, NymOfflineSignersContractError> {
        let Some(active_proposal_id) = self.active_proposals.may_load(storage, signer)? else {
            return Ok(None);
        };
        self.proposals
            .may_load(storage, active_proposal_id)
            .map_err(Into::into)
    }

    fn recently_marked_online(
        &self,
        storage: &dyn Storage,
        env: &Env,
        signer: &Addr,
    ) -> Result<bool, NymOfflineSignersContractError> {
        let Some(last_status_reset) = self.last_status_reset.may_load(storage, signer)? else {
            return Ok(false);
        };

        let config = self.config.load(storage)?;
        Ok(
            last_status_reset
                .recently_marked_online(&env.block, config.status_change_cooldown_secs),
        )
    }

    fn recently_marked_offline(
        &self,
        storage: &dyn Storage,
        env: &Env,
        signer: &Addr,
    ) -> Result<bool, NymOfflineSignersContractError> {
        let Some(signer_info) = self.offline_signers.may_load(storage, signer)? else {
            return Ok(false);
        };

        let config = self.config.load(storage)?;
        Ok(signer_info.recently_marked_offline(&env.block, config.status_change_cooldown_secs))
    }

    pub(crate) fn proposal_expired(
        &self,
        storage: &dyn Storage,
        env: &Env,
        proposal: &Proposal,
    ) -> Result<bool, NymOfflineSignersContractError> {
        let config = self.config.load(storage)?;
        Ok(proposal.expired(&env.block, config.maximum_proposal_lifetime_secs))
    }

    fn try_vote(
        &self,
        storage: &mut dyn Storage,
        env: &Env,
        proposer: &Addr,
        signer: &Addr,
    ) -> Result<ProposalId, NymOfflineSignersContractError> {
        // 1. retrieve existing proposal or make a new one
        let active_proposal_id = match self.try_load_active_proposal(storage, signer)? {
            Some(existing) => {
                // 1.1. check if proposal has already expired
                if self.proposal_expired(storage, env, &existing)? {
                    // 1.2.1. remake the proposal
                    self.insert_new_active_proposal(storage, env, proposer, signer)?
                } else {
                    // 1.2.2. use the existing proposal
                    existing.id
                }
            }
            None => self.insert_new_active_proposal(storage, env, proposer, signer)?,
        };

        let vote = (active_proposal_id, proposer);

        // 2. check if this vote already exists
        // (technically we could ignore this, but it shouldn't occur anyway)
        if self.votes.may_load(storage, vote)?.is_some() {
            return Err(NymOfflineSignersContractError::AlreadyVoted {
                voter: proposer.clone(),
                proposal: active_proposal_id,
                target: signer.clone(),
            });
        }

        // 3. save the vote
        self.votes
            .save(storage, vote, &VoteInformation::new(&env.block))?;

        Ok(active_proposal_id)
    }

    fn total_votes(&self, storage: &dyn Storage, proposal_id: ProposalId) -> u32 {
        self.votes
            .prefix(proposal_id)
            .range_raw(storage, None, None, Order::Ascending)
            .count() as u32
    }

    pub(crate) fn add_proposal_resolution(
        &self,
        deps: Deps,
        env: &Env,
        proposal: Proposal,
    ) -> Result<ProposalWithResolution, NymOfflineSignersContractError> {
        Ok(ProposalWithResolution {
            passed: NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE.proposal_passed(
                deps,
                proposal.id,
                None,
            )?,
            voting_finished: NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE.proposal_expired(
                deps.storage,
                env,
                &proposal,
            )?,
            proposal,
        })
    }

    pub(crate) fn proposal_passed(
        &self,
        deps: Deps,
        proposal_id: ProposalId,
        group_contract: Option<Cw4Contract>,
    ) -> Result<bool, NymOfflineSignersContractError> {
        let group_contract = match group_contract {
            Some(group_contract) => group_contract,
            None => {
                let dkg_contract_address = self.dkg_contract.load(deps.storage)?;
                Cw4Contract::new(
                    deps.querier
                        .query_dkg_cw4_contract_address(dkg_contract_address)?,
                )
            }
        };
        // obtain the total number of group members (i.e. eligible voters)
        let eligible_voters = group_members(&deps.querier, &group_contract)?;

        let config = self.config.load(deps.storage)?;
        let required_quorum = config.required_quorum;

        // get the vote count and determine the ratio
        let votes = self.total_votes(deps.storage, proposal_id);
        let vote_ratio = Decimal::from_ratio(votes, eligible_voters);

        // check if we passed quorum
        if vote_ratio >= required_quorum {
            return Ok(true);
        }

        Ok(false)
    }

    fn finalize_vote(
        &self,
        deps: DepsMut,
        env: &Env,
        proposal_id: ProposalId,
        marked_signer: &Addr,
        group_contract: Cw4Contract,
    ) -> Result<bool, NymOfflineSignersContractError> {
        // check if the signer hasn't already been marked as offline and this is just an additional vote
        if self
            .offline_signers
            .may_load(deps.storage, marked_signer)?
            .is_some()
        {
            return Ok(true);
        }

        // check if we passed quorum
        if self.proposal_passed(deps.as_ref(), proposal_id, Some(group_contract))? {
            self.offline_signers.save(
                deps.storage,
                marked_signer,
                &OfflineSignerInformation {
                    marked_offline_at: env.block.clone(),
                    associated_proposal: proposal_id,
                },
                env.block.height,
            )?;
            return Ok(true);
        }

        Ok(false)
    }

    pub fn propose_or_vote(
        &self,
        deps: DepsMut,
        env: Env,
        proposer: Addr,
        signer: Addr,
    ) -> Result<bool, NymOfflineSignersContractError> {
        let dkg_contract_address = self.dkg_contract.load(deps.storage)?;
        let group_contract = Cw4Contract::new(
            deps.querier
                .query_dkg_cw4_contract_address(dkg_contract_address)?,
        );

        // 1. check if the proposer is a valid DKG CW4 group member
        if group_contract
            .is_voting_member(&deps.querier, &proposer, None)?
            .is_none()
        {
            return Err(NymOfflineSignersContractError::NotGroupMember {
                address: proposer.clone(),
            });
        }

        // 2. check if the proposed signer is a valid DKG CW4 group member
        if group_contract
            .is_voting_member(&deps.querier, &signer, None)?
            .is_none()
        {
            return Err(NymOfflineSignersContractError::NotGroupMember {
                address: signer.clone(),
            });
        }

        // 3. check if the signer hasn't recently been marked as online
        // (to prevent constant switching between online and offline)
        if self.recently_marked_online(deps.storage, &env, &signer)? {
            return Err(NymOfflineSignersContractError::RecentlyCameOnline {
                address: signer.clone(),
            });
        }

        // 4. try to apply the vote
        let proposal_id = self.try_vote(deps.storage, &env, &proposer, &signer)?;

        // 5. check if quorum is reached
        let reached_quorum =
            self.finalize_vote(deps, &env, proposal_id, &signer, group_contract)?;

        Ok(reached_quorum)
    }

    pub fn reset_offline_status(
        &self,
        deps: DepsMut,
        env: Env,
        sender: Addr,
    ) -> Result<(), NymOfflineSignersContractError> {
        // 1. check if this sender hasn't been marked offline recently
        // (to prevent constant switching between online and offline)
        if self.recently_marked_offline(deps.storage, &env, &sender)? {
            return Err(NymOfflineSignersContractError::RecentlyCameOffline { address: sender });
        }

        // 2. an offline signer (or a singer in the process of being marked as offline) must have an active
        // proposal going against them, if it doesn't exist, return an error
        if !self.active_proposals.has(deps.storage, &sender) {
            return Err(NymOfflineSignersContractError::NotOffline { address: sender });
        }

        // 3. reset proposal and offline status
        self.active_proposals.remove(deps.storage, &sender);
        self.offline_signers
            .remove(deps.storage, &sender, env.block.height)?;

        // 4. update online metadata
        self.last_status_reset.save(
            deps.storage,
            &sender,
            &StatusResetInformation {
                status_reset_at: env.block.clone(),
            },
        )?;

        Ok(())
    }
}

pub mod retrieval_limits {
    pub const ACTIVE_PROPOSALS_DEFAULT_LIMIT: u32 = 25;
    pub const ACTIVE_PROPOSALS_MAX_LIMIT: u32 = 50;

    pub const PROPOSALS_DEFAULT_LIMIT: u32 = 50;
    pub const PROPOSALS_MAX_LIMIT: u32 = 100;

    pub const VOTES_DEFAULT_LIMIT: u32 = 50;
    pub const VOTES_MAX_LIMIT: u32 = 100;

    pub const OFFLINE_SIGNERS_DEFAULT_LIMIT: u32 = 50;
    pub const OFFLINE_SIGNERS_MAX_LIMIT: u32 = 100;

    pub const LAST_STATUS_RESET_DEFAULT_LIMIT: u32 = 50;
    pub const LAST_STATUS_RESET_MAX_LIMIT: u32 = 100;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod performance_contract_storage {
        use super::*;

        #[cfg(test)]
        mod initialisation {
            use super::*;
            use cosmwasm_std::testing::mock_env;
            use nym_contracts_common_testing::mock_dependencies;

            #[test]
            fn sets_contract_admin() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin1 = deps.api.addr_make("first-admin");
                let admin2 = deps.api.addr_make("second-admin");

                storage.initialise(deps.as_mut(), env.clone(), admin1.clone())?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin1).is_ok());

                storage.initialise(deps.as_mut(), env.clone(), admin2.clone())?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin2).is_ok());

                Ok(())
            }
        }
    }
}
