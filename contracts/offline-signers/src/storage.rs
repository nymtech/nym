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
    VoteInformation,
};

pub const NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE: NymOfflineSignersStorage =
    NymOfflineSignersStorage::new();

pub struct NymOfflineSignersStorage {
    pub(crate) contract_admin: Admin,
    pub(crate) dkg_contract: Item<Addr>,

    // configurable (by the admin) values of this contract
    pub(crate) config: Item<Config>,

    // map between given signer and a currently active (if applicable) proposal id
    // note: one signer can have only a single active proposal against them at a given time
    pub(crate) active_proposals: Map<&'static Addr, ProposalId>,

    // all proposals ever created - realistically we'll ever see a handful of them,
    // so leaving them is fine
    pub(crate) proposals: Map<ProposalId, Proposal>,

    // votes information
    pub(crate) votes: Map<(ProposalId, &'static Addr), VoteInformation>,

    // map of all signers marked as offline
    pub(crate) offline_signers: SnapshotMap<&'static Addr, OfflineSignerInformation>,

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
            proposal_count: Item::new(PROPOSAL_COUNT),
        }
    }

    fn next_proposal_id(&self, storage: &mut dyn Storage) -> StdResult<ProposalId> {
        let id: ProposalId = self.proposal_count.may_load(storage)?.unwrap_or_default() + 1;
        self.proposal_count.save(storage, &id)?;
        Ok(id)
    }

    fn new_active_proposal(
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

    fn recently_marked_online(
        &self,
        signer: &Addr,
    ) -> Result<bool, NymOfflineSignersContractError> {
        todo!()
    }

    fn try_load_active_proposal(
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
                let config = self.config.load(storage)?;

                // 1.1. check if proposal has already expired
                if existing.expired(&env.block, config.maximum_proposal_lifetime_secs) {
                    // 1.2.1. remake the proposal
                    self.new_active_proposal(storage, env, proposer, signer)?
                } else {
                    // 1.2.2. use the existing proposal
                    existing.id
                }
            }
            None => self.new_active_proposal(storage, env, proposer, signer)?,
        };

        let vote = (active_proposal_id, proposer);

        // 2. check if this vote already exists
        // (technically we could ignore this, but it shouldn't occur anyway)
        if self.votes.may_load(storage, vote)?.is_some() {
            todo!("duplicate vote")
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

    fn finalize_vote(
        &self,
        storage: &mut dyn Storage,
        env: &Env,
        proposal_id: ProposalId,
        marked_signer: &Addr,
        eligible_voters: u32,
    ) -> Result<bool, NymOfflineSignersContractError> {
        let config = self.config.load(storage)?;

        let required_quorum = config.required_quorum;

        let votes = self.total_votes(storage, proposal_id);
        let vote_ratio = Decimal::from_ratio(votes, eligible_voters);

        // check if we passed quorum
        if vote_ratio >= required_quorum {
            self.offline_signers.save(
                storage,
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
        mut deps: DepsMut,
        env: Env,
        proposer: Addr,
        signer: Addr,
    ) -> Result<bool, NymOfflineSignersContractError> {
        let dkg_group_address = self.dkg_contract.load(deps.storage)?;
        let group_contract = Cw4Contract::new(
            deps.querier
                .query_dkg_cw4_contract_address(dkg_group_address)?,
        );

        // 1. check if the proposer is a valid DKG CW4 group member
        if group_contract
            .is_voting_member(&deps.querier, &proposer, None)?
            .is_none()
        {
            todo!("proposer not authorised")
        }

        // 2. check if the proposed signer is a valid DKG CW4 group member
        if group_contract
            .is_voting_member(&deps.querier, &signer, None)?
            .is_none()
        {
            todo!("proposed signer is not authorised")
        }

        // 3. check if the proposed signer is already marked as offline
        if self
            .offline_signers
            .may_load(deps.storage, &signer)?
            .is_some()
        {
            todo!("already marked as offline")
        }

        // 4. check if the proposed signer has recently been marked as online
        // (this is to prevent attacks by smaller threshold on properly working instances)
        if self.recently_marked_online(&signer)? {
            todo!("too soon for another proposal")
        }

        // 5. try apply the vote
        let proposal_id = self.try_vote(deps.storage, &env, &proposer, &signer)?;

        let total_members = group_members(&deps.querier, &group_contract)?;

        // 6. check if quorum is reached
        let reached_qourum =
            self.finalize_vote(deps.storage, &env, proposal_id, &signer, total_members)?;

        Ok(reached_qourum)
    }
}

pub mod retrieval_limits {
    //
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
