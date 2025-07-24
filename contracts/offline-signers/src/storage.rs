// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::{group_members, DkgContractQuerier};
use cosmwasm_std::{Addr, Decimal, Deps, DepsMut, Env, Order, StdResult, Storage};
use cw4::Cw4Contract;
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map, SnapshotItem, Strategy};
use nym_offline_signers_contract_common::constants::storage_keys;
use nym_offline_signers_contract_common::constants::storage_keys::PROPOSAL_COUNT;
use nym_offline_signers_contract_common::{
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

    // details on signers marked as offline
    pub(crate) offline_signers: OfflineSignersStorage,

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
            offline_signers: OfflineSignersStorage::new(),
            last_status_reset: Map::new(storage_keys::LAST_STATUS_RESET),
            proposal_count: Item::new(PROPOSAL_COUNT),
        }
    }

    fn next_proposal_id(&self, storage: &mut dyn Storage) -> StdResult<ProposalId> {
        let id: ProposalId = self.proposal_count.may_load(storage)?.unwrap_or_default() + 1;
        self.proposal_count.save(storage, &id)?;
        Ok(id)
    }

    pub(crate) fn insert_new_active_proposal(
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
        self.active_proposals
            .save(storage, proposed_offline_signer, &id)?;
        Ok(id)
    }

    #[cfg(test)]
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
        dkg_contract_address: Addr,
        config: Config,
    ) -> Result<(), NymOfflineSignersContractError> {
        // set the dkg contract address
        self.dkg_contract
            .save(deps.storage, &dkg_contract_address)?;

        // check quorum and set config values
        if config.required_quorum > Decimal::one() {
            return Err(NymOfflineSignersContractError::RequiredQuorumBiggerThanOne);
        }
        self.config.save(deps.storage, &config)?;

        // set the contract admin
        self.contract_admin
            .set(deps.branch(), Some(admin.clone()))?;

        // finally initialise the inner offline signers storage wrapper
        self.offline_signers.initialise(deps, env)
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
        let Some(signer_info) = self
            .offline_signers
            .load_signer_information(storage, signer)?
        else {
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
        let eligible_voters = group_members(&deps.querier, &group_contract)?.len() as u32;

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
            .has_signer_information(deps.storage, marked_signer)
        {
            return Ok(true);
        }

        // check if we passed quorum
        if self.proposal_passed(deps.as_ref(), proposal_id, Some(group_contract))? {
            self.offline_signers.insert_offline_signer_information(
                deps.storage,
                env,
                marked_signer,
                &OfflineSignerInformation {
                    marked_offline_at: env.block.clone(),
                    associated_proposal: proposal_id,
                },
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
            .remove_offline_signer_information(deps.storage, &env, &sender)?;

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

pub struct OfflineSignersStorage {
    // map of all signers marked as offline
    pub(crate) information: Map<&'static Addr, OfflineSignerInformation>,

    // list of addresses of signers currently marked as offline
    // we need a separate entry to be able to retrieve list of signers marked at particular height.
    // given that we won't ever have more than ~20 entries, loading and resaving the whole vec is not a problem
    pub(crate) addresses: SnapshotItem<Vec<Addr>>,
}

impl OfflineSignersStorage {
    #[allow(clippy::new_without_default)]
    pub(crate) const fn new() -> Self {
        OfflineSignersStorage {
            information: Map::new(storage_keys::OFFLINE_SIGNERS_INFORMATION),
            addresses: SnapshotItem::new(
                storage_keys::OFFLINE_SIGNERS,
                storage_keys::OFFLINE_SIGNERS_CHECKPOINTS,
                storage_keys::OFFLINE_SIGNERS_CHANGELOG,
                Strategy::EveryBlock,
            ),
        }
    }

    fn initialise(&self, deps: DepsMut, env: Env) -> Result<(), NymOfflineSignersContractError> {
        self.addresses
            .save(deps.storage, &Vec::new(), env.block.height)?;
        Ok(())
    }

    pub(crate) fn load_signer_information(
        &self,
        storage: &dyn Storage,
        signer: &Addr,
    ) -> Result<Option<OfflineSignerInformation>, NymOfflineSignersContractError> {
        self.information
            .may_load(storage, signer)
            .map_err(Into::into)
    }

    pub(crate) fn has_signer_information(&self, storage: &dyn Storage, signer: &Addr) -> bool {
        self.information.has(storage, signer)
    }

    pub(crate) fn insert_offline_signer_information(
        &self,
        storage: &mut dyn Storage,
        env: &Env,
        signer: &Addr,
        info: &OfflineSignerInformation,
    ) -> Result<(), NymOfflineSignersContractError> {
        // insert details into the map
        self.information.save(storage, signer, info)?;

        // update the snapshot
        let mut all_signers = self.addresses.load(storage)?;
        all_signers.push(signer.clone());
        self.addresses
            .save(storage, &all_signers, env.block.height)?;
        Ok(())
    }

    pub(crate) fn remove_offline_signer_information(
        &self,
        storage: &mut dyn Storage,
        env: &Env,
        signer: &Addr,
    ) -> Result<(), NymOfflineSignersContractError> {
        // remove details from the map
        self.information.remove(storage, signer);

        // update the snapshot
        let mut all_signers = self.addresses.load(storage)?;
        if let Some(pos) = all_signers.iter().position(|x| x == signer) {
            all_signers.remove(pos);
        }
        self.addresses
            .save(storage, &all_signers, env.block.height)?;
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
    use nym_offline_signers_contract_common::InstantiateMsg;

    fn init_with_quorum(required_quorum: Decimal) -> InstantiateMsg {
        InstantiateMsg {
            dkg_contract_address: "".to_string(),
            config: Config {
                required_quorum,
                ..Default::default()
            },
        }
    }

    #[cfg(test)]
    mod offline_signers_contract_storage {
        use super::*;
        use crate::testing::{
            init_contract_tester, init_custom_contract_tester, OfflineSignersContractTesterExt,
        };
        use cosmwasm_std::testing::mock_env;
        use nym_contracts_common_testing::{ChainOpts, ContractOpts};

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
                let dummy_dkg_contract = deps.api.addr_make("dkg-contract");
                let config = Config::default();

                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin1.clone(),
                    dummy_dkg_contract.clone(),
                    config,
                )?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin1).is_ok());

                let mut deps = mock_dependencies();
                storage.initialise(
                    deps.as_mut(),
                    env,
                    admin2.clone(),
                    dummy_dkg_contract,
                    config,
                )?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin2).is_ok());

                Ok(())
            }

            #[test]
            fn sets_dkg_contract_address() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin = deps.api.addr_make("admin");
                let dummy_dkg_contract1 = deps.api.addr_make("dkg-contract1");
                let dummy_dkg_contract2 = deps.api.addr_make("dkg-contract2");
                let config = Config::default();

                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin.clone(),
                    dummy_dkg_contract1.clone(),
                    config,
                )?;
                assert_eq!(
                    storage.dkg_contract.load(deps.as_ref().storage)?,
                    dummy_dkg_contract1
                );

                let mut deps = mock_dependencies();
                storage.initialise(
                    deps.as_mut(),
                    env,
                    admin,
                    dummy_dkg_contract2.clone(),
                    config,
                )?;
                assert_eq!(
                    storage.dkg_contract.load(deps.as_ref().storage)?,
                    dummy_dkg_contract2
                );
                Ok(())
            }

            #[test]
            fn forbids_invalid_quorum_value() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin = deps.api.addr_make("admin");
                let dummy_dkg_contract = deps.api.addr_make("dkg-contract");
                let bad_config1 = Config {
                    required_quorum: Decimal::percent(666666),
                    ..Default::default()
                };

                let bad_config2 = Config {
                    required_quorum: Decimal::percent(101),
                    ..Default::default()
                };

                let borderline_good_config = Config {
                    required_quorum: Decimal::percent(100),
                    ..Default::default()
                };

                let good_config = Config {
                    required_quorum: Decimal::percent(69),
                    ..Default::default()
                };

                let res = storage
                    .initialise(
                        deps.as_mut(),
                        env.clone(),
                        admin.clone(),
                        dummy_dkg_contract.clone(),
                        bad_config1,
                    )
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymOfflineSignersContractError::RequiredQuorumBiggerThanOne
                );

                let res = storage
                    .initialise(
                        deps.as_mut(),
                        env.clone(),
                        admin.clone(),
                        dummy_dkg_contract.clone(),
                        bad_config2,
                    )
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymOfflineSignersContractError::RequiredQuorumBiggerThanOne
                );

                let res = storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin.clone(),
                    dummy_dkg_contract.clone(),
                    borderline_good_config,
                );
                assert!(res.is_ok());

                let mut deps = mock_dependencies();
                let res = storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin.clone(),
                    dummy_dkg_contract.clone(),
                    good_config,
                );
                assert!(res.is_ok());

                Ok(())
            }

            #[test]
            fn initialises_internal_offline_signers_storage() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin = deps.api.addr_make("admin");
                let dummy_dkg_contract = deps.api.addr_make("dkg-contract");

                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin.clone(),
                    dummy_dkg_contract.clone(),
                    Config::default(),
                )?;

                // this checks that the empty vec has actually been saved (as opposed to value not existing at all)
                assert!(OfflineSignersStorage::new()
                    .addresses
                    .load(deps.as_ref().storage)?
                    .is_empty());

                Ok(())
            }
        }

        #[test]
        #[allow(clippy::panic)]
        fn try_load_active_proposal() -> anyhow::Result<()> {
            let storage = NymOfflineSignersStorage::new();
            let mut tester = init_contract_tester();

            let signer = tester.random_group_member();
            let proposer = tester.random_group_member();

            assert!(storage
                .try_load_active_proposal(&tester, &signer)?
                .is_none());

            let env = mock_env();
            storage.propose_or_vote(tester.deps_mut(), env, proposer.clone(), signer.clone())?;

            // this was the first proposal
            let Some(proposal) = storage.try_load_active_proposal(&tester, &signer)? else {
                panic!("test failure")
            };

            assert_eq!(proposal.id, 1);
            assert_eq!(proposal.proposed_offline_signer, signer);
            assert_eq!(proposal.proposer, proposer);

            Ok(())
        }

        #[test]
        fn recently_marked_online() -> anyhow::Result<()> {
            let storage = NymOfflineSignersStorage::new();
            let mut tester = init_contract_tester();

            let signer = tester.random_group_member();

            // not even offline
            assert!(!storage.recently_marked_online(&tester, &tester.env(), &signer)?);

            tester.insert_offline_signer(&signer);

            // offline
            assert!(!storage.recently_marked_online(&tester, &tester.env(), &signer)?);

            // JUST marked online
            tester.advance_day_of_blocks();
            tester.reset_offline_status(&signer);
            assert!(storage.recently_marked_online(&tester, &tester.env(), &signer)?);

            // few blocks passed (still below threshold);
            tester.next_block();
            tester.next_block();
            tester.next_block();
            tester.next_block();
            assert!(storage.recently_marked_online(&tester, &tester.env(), &signer)?);

            // threshold has passed
            tester.advance_day_of_blocks();
            assert!(!storage.recently_marked_online(&tester, &tester.env(), &signer)?);

            // offline again
            tester.insert_offline_signer(&signer);
            assert!(!storage.recently_marked_online(&tester, &tester.env(), &signer)?);

            // and online again
            tester.advance_day_of_blocks();
            tester.reset_offline_status(&signer);
            assert!(storage.recently_marked_online(&tester, &tester.env(), &signer)?);

            Ok(())
        }

        #[test]
        fn recently_marked_offline() -> anyhow::Result<()> {
            let storage = NymOfflineSignersStorage::new();
            let mut tester = init_contract_tester();

            let signer = tester.random_group_member();

            assert!(!storage.recently_marked_offline(&tester, &tester.env(), &signer)?);

            tester.insert_offline_signer(&signer);

            assert!(storage.recently_marked_offline(&tester, &tester.env(), &signer)?);

            // few blocks passed (still below threshold);
            tester.next_block();
            tester.next_block();
            tester.next_block();
            tester.next_block();
            assert!(storage.recently_marked_offline(&tester, &tester.env(), &signer)?);

            // threshold has passed
            tester.advance_day_of_blocks();
            assert!(!storage.recently_marked_offline(&tester, &tester.env(), &signer)?);

            // came back online
            tester.reset_offline_status(&signer);
            assert!(!storage.recently_marked_offline(&tester, &tester.env(), &signer)?);

            tester.advance_day_of_blocks();
            // offline again
            tester.insert_offline_signer(&signer);
            assert!(storage.recently_marked_offline(&tester, &tester.env(), &signer)?);

            // again threshold has passed
            tester.advance_day_of_blocks();
            assert!(!storage.recently_marked_offline(&tester, &tester.env(), &signer)?);

            Ok(())
        }

        #[test]
        fn proposal_expired() -> anyhow::Result<()> {
            let storage = NymOfflineSignersStorage::new();
            let mut tester = init_contract_tester();

            let signer = tester.random_group_member();

            let threshold = storage.config.load(&tester)?.maximum_proposal_lifetime_secs;

            let proposal_id = tester.make_proposal(&signer);
            let proposal = storage.proposals.load(&tester, proposal_id)?;

            let initial_time = tester.env().block.time;

            assert!(!storage.proposal_expired(&tester, &tester.env(), &proposal)?);

            tester.next_block();
            assert!(!storage.proposal_expired(&tester, &tester.env(), &proposal)?);
            tester.next_block();
            assert!(!storage.proposal_expired(&tester, &tester.env(), &proposal)?);

            tester.set_block_time(initial_time.plus_seconds(threshold - 1));
            assert!(!storage.proposal_expired(&tester, &tester.env(), &proposal)?);

            tester.set_block_time(initial_time.plus_seconds(threshold));
            assert!(storage.proposal_expired(&tester, &tester.env(), &proposal)?);

            tester.advance_day_of_blocks();
            assert!(storage.proposal_expired(&tester, &tester.env(), &proposal)?);

            Ok(())
        }

        #[cfg(test)]
        mod try_vote {
            use super::*;
            use crate::testing::{init_contract_tester, OfflineSignersContractTesterExt};
            use nym_contracts_common_testing::{ChainOpts, ContractOpts, FullReader};

            #[test]
            fn proposal_reuse() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut tester = init_contract_tester();

                let member1 = tester.random_group_member();
                let member2 = tester.random_group_member();
                let member3 = tester.random_group_member();

                let voter = tester.random_group_member();

                // sanity check due to RNG : )
                // if those ever fail, call `init_contract_tester_with_group_members`
                // and provide higher than default value there until rngesus smiles at you
                assert_ne!(member1, member2);
                assert_ne!(member1, member3);
                assert_ne!(member2, member3);

                let existing_expired_original = tester.make_proposal(&member1);
                // advance blocks so that the proposal would have already expired
                tester.advance_day_of_blocks();

                let existing_not_expired_original = tester.make_proposal(&member2);

                let env = tester.env();

                // ## TEST SETUP END

                // existing proposal that has already expired
                let res_expired = storage.try_vote(tester.storage_mut(), &env, &voter, &member1)?;

                // existing proposal that has NOT yet expired
                let res_not_expired =
                    storage.try_vote(tester.storage_mut(), &env, &voter, &member2)?;

                // no existing proposal
                let res_new = storage.try_vote(tester.storage_mut(), &env, &voter, &member3)?;

                // the same proposal has been used
                assert_eq!(res_not_expired, existing_not_expired_original);
                // new proposal has been created
                assert_ne!(res_expired, existing_expired_original);

                let all_proposals = storage.proposals.all_values(&tester)?;
                // we expect 4 proposals:
                // - the original expired one
                // - the non-expired old one
                // - the recreated expired one
                // - proposal for new signer
                assert_eq!(all_proposals.len(), 4);

                // votes are actually saved
                assert!(storage
                    .votes
                    .has(&tester, (existing_not_expired_original, &voter)));
                assert!(storage.votes.has(&tester, (res_expired, &voter)));
                assert!(storage.votes.has(&tester, (res_new, &voter)));
                assert!(!storage
                    .votes
                    .has(&tester, (existing_expired_original, &voter)));

                Ok(())
            }

            #[test]
            fn duplicate_votes_are_rejected() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut tester = init_contract_tester();

                let member1 = tester.random_group_member();
                let member2 = tester.random_group_member();
                let voter1 = tester.random_group_member();
                let voter2 = tester.random_group_member();

                // sanity check due to RNG : )
                // if those ever fail, call `init_contract_tester_with_group_members`
                // and provide higher than default value there until rngesus smiles at you
                assert_ne!(member1, member2);
                assert_ne!(voter1, voter2);

                assert_ne!(member1, voter1);
                assert_ne!(member2, voter1);

                assert_ne!(member1, voter2);
                assert_ne!(member2, voter2);

                let env = tester.env();

                // first vote
                assert!(storage
                    .try_vote(tester.storage_mut(), &env, &voter1, &member1)
                    .is_ok());

                // second vote for the same signer is rejected
                assert_eq!(
                    storage
                        .try_vote(tester.storage_mut(), &env, &voter1, &member1)
                        .unwrap_err(),
                    NymOfflineSignersContractError::AlreadyVoted {
                        voter: voter1.clone(),
                        proposal: 1,
                        target: member1.clone(),
                    }
                );

                // but is fine from another voter
                assert!(storage
                    .try_vote(tester.storage_mut(), &env, &voter2, &member1)
                    .is_ok());

                // or towards another signer
                assert!(storage
                    .try_vote(tester.storage_mut(), &env, &voter1, &member2)
                    .is_ok());

                // it is also fine after proposal gets implicitly recreated due to expiration
                tester.advance_day_of_blocks();
                let env = tester.env();
                assert!(storage
                    .try_vote(tester.storage_mut(), &env, &voter1, &member1)
                    .is_ok());

                Ok(())
            }
        }

        #[test]
        fn total_votes() -> anyhow::Result<()> {
            let storage = NymOfflineSignersStorage::new();
            let mut tester = init_contract_tester();

            let target1 = tester.random_group_member();
            let target2 = tester.random_group_member();
            assert_ne!(target1, target2);

            let proposal1 = tester.insert_empty_proposal(&target1);
            let proposal2 = tester.insert_empty_proposal(&target2);

            let all_voters = tester.group_members();
            for (i, voter) in all_voters.iter().enumerate() {
                let env = tester.env();

                assert_eq!(
                    storage.total_votes(tester.storage_mut(), proposal1),
                    i as u32
                );
                storage.try_vote(tester.storage_mut(), &env, voter, &target1)?;
                assert_eq!(
                    storage.total_votes(tester.storage_mut(), proposal1),
                    (i + 1) as u32
                );
                assert_eq!(storage.total_votes(tester.storage_mut(), proposal2), 0);
            }

            for (i, voter) in all_voters.iter().enumerate() {
                let env = tester.env();

                assert_eq!(
                    storage.total_votes(tester.storage_mut(), proposal2),
                    i as u32
                );
                storage.try_vote(tester.storage_mut(), &env, voter, &target2)?;
                assert_eq!(
                    storage.total_votes(tester.storage_mut(), proposal2),
                    (i + 1) as u32
                );
                assert_eq!(
                    storage.total_votes(tester.storage_mut(), proposal1),
                    all_voters.len() as u32
                );
            }

            Ok(())
        }

        #[test]
        // check requires votes / eligible_voters >= quorum
        fn proposal_passed() -> anyhow::Result<()> {
            let storage = NymOfflineSignersStorage::new();
            let mut tester_10q =
                init_custom_contract_tester(10, init_with_quorum(Decimal::percent(10)));
            let mut tester_25q =
                init_custom_contract_tester(10, init_with_quorum(Decimal::percent(25)));
            let mut tester_100q =
                init_custom_contract_tester(10, init_with_quorum(Decimal::percent(100)));

            // check proposal that doesn't exist
            assert!(!storage.proposal_passed(tester_10q.deps(), 69, None)?);

            // those values are the same for all testers
            let target = tester_10q.random_group_member();
            let all_voters = tester_10q.group_members();

            // make dummy_proposals
            let p_10q = tester_10q.insert_empty_proposal(&target);
            let p_25q = tester_25q.insert_empty_proposal(&target);
            let p_100q = tester_100q.insert_empty_proposal(&target);

            // initially no proposal has been passed
            assert!(!storage.proposal_passed(tester_10q.deps(), p_10q, None)?);
            assert!(!storage.proposal_passed(tester_25q.deps(), p_25q, None)?);
            assert!(!storage.proposal_passed(tester_100q.deps(), p_100q, None)?);

            // add first vote
            tester_10q.add_vote(p_10q, &all_voters[0]);
            tester_25q.add_vote(p_25q, &all_voters[0]);
            tester_100q.add_vote(p_100q, &all_voters[0]);

            // in the case of the first tester (where quorum is 10%), it should now be passed
            assert!(storage.proposal_passed(tester_10q.deps(), p_10q, None)?);
            assert!(!storage.proposal_passed(tester_25q.deps(), p_25q, None)?);
            assert!(!storage.proposal_passed(tester_100q.deps(), p_100q, None)?);

            // another vote
            tester_10q.add_vote(p_10q, &all_voters[1]);
            tester_25q.add_vote(p_25q, &all_voters[1]);
            tester_100q.add_vote(p_100q, &all_voters[1]);

            // with more votes, first proposal is still marked as passed
            assert!(storage.proposal_passed(tester_10q.deps(), p_10q, None)?);
            assert!(!storage.proposal_passed(tester_25q.deps(), p_25q, None)?);
            assert!(!storage.proposal_passed(tester_100q.deps(), p_100q, None)?);

            // with third vote (30%) second tester has passed its proposal that required 25% quorum)
            tester_25q.add_vote(p_25q, &all_voters[2]);
            tester_100q.add_vote(p_100q, &all_voters[2]);

            assert!(storage.proposal_passed(tester_25q.deps(), p_25q, None)?);
            assert!(!storage.proposal_passed(tester_100q.deps(), p_100q, None)?);

            // last proposal won't be passed until all voters have voted
            for voter in all_voters[3..=8].iter() {
                tester_100q.add_vote(p_100q, voter);
                assert!(!storage.proposal_passed(tester_100q.deps(), p_100q, None)?);
            }
            tester_100q.add_vote(p_100q, &all_voters[9]);
            assert!(storage.proposal_passed(tester_100q.deps(), p_100q, None)?);

            Ok(())
        }

        #[test]
        fn finalize_vote() -> anyhow::Result<()> {
            fn mock_vote_information() -> VoteInformation {
                VoteInformation {
                    voted_at: mock_env().block,
                }
            }

            let storage = NymOfflineSignersStorage::new();
            let mut tester =
                init_custom_contract_tester(10, init_with_quorum(Decimal::percent(20)));

            let target = tester.random_group_member();
            let all_voters = tester.group_members();
            let proposal = tester.insert_empty_proposal(&target);
            let group_contract = tester.group_contract_wrapper();
            let env = mock_env();

            // first vote (no quorum yet)
            storage.votes.save(
                tester.storage_mut(),
                (proposal, &all_voters[0]),
                &mock_vote_information(),
            )?;
            let got_quorum = storage.finalize_vote(
                tester.deps_mut(),
                &env,
                proposal,
                &target,
                group_contract.clone(),
            )?;
            assert!(!storage
                .offline_signers
                .has_signer_information(&tester, &target));
            assert!(!got_quorum);

            // second vote (reached quorum!)
            storage.votes.save(
                tester.storage_mut(),
                (proposal, &all_voters[1]),
                &mock_vote_information(),
            )?;
            let got_quorum = storage.finalize_vote(
                tester.deps_mut(),
                &env,
                proposal,
                &target,
                group_contract.clone(),
            )?;
            assert!(got_quorum);
            assert!(storage
                .offline_signers
                .has_signer_information(&tester, &target));

            // third vote (already passed quorum before)
            storage.votes.save(
                tester.storage_mut(),
                (proposal, &all_voters[2]),
                &mock_vote_information(),
            )?;
            let got_quorum = storage.finalize_vote(
                tester.deps_mut(),
                &env,
                proposal,
                &target,
                group_contract,
            )?;
            assert!(got_quorum);
            assert!(storage
                .offline_signers
                .has_signer_information(&tester, &target));

            Ok(())
        }

        #[cfg(test)]
        mod propose_or_vote {
            use super::*;
            use itertools::Itertools;
            use nym_contracts_common_testing::RandExt;

            #[test]
            fn proposer_has_to_be_dkg_group_member() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut tester = init_contract_tester();

                let signer = tester.random_group_member();
                let bad_proposer = tester.generate_account();
                let good_proposer = tester.random_group_member();

                let env = tester.env();
                let err = storage
                    .propose_or_vote(tester.deps_mut(), env, bad_proposer.clone(), signer.clone())
                    .unwrap_err();
                assert_eq!(
                    err,
                    NymOfflineSignersContractError::NotGroupMember {
                        address: bad_proposer
                    }
                );

                let env = tester.env();
                let res = storage.propose_or_vote(tester.deps_mut(), env, good_proposer, signer);

                assert!(res.is_ok());
                Ok(())
            }

            #[test]
            fn proposed_signer_has_to_be_dkg_group_member() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut tester = init_contract_tester();

                let good_signer = tester.random_group_member();
                let bad_signer = tester.generate_account();
                let proposer = tester.random_group_member();

                let env = tester.env();
                let err = storage
                    .propose_or_vote(tester.deps_mut(), env, proposer.clone(), bad_signer.clone())
                    .unwrap_err();
                assert_eq!(
                    err,
                    NymOfflineSignersContractError::NotGroupMember {
                        address: bad_signer
                    }
                );

                let env = tester.env();
                let res = storage.propose_or_vote(tester.deps_mut(), env, proposer, good_signer);

                assert!(res.is_ok());
                Ok(())
            }

            #[test]
            fn signer_must_have_not_recently_come_back_online() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut tester = init_contract_tester();

                let signer = tester.random_group_member();
                let proposer = tester.random_group_member();

                tester.insert_offline_signer(&signer);
                tester.advance_day_of_blocks();
                tester.reset_offline_status(&signer);

                let env = tester.env();
                let err = storage
                    .propose_or_vote(tester.deps_mut(), env, proposer.clone(), signer.clone())
                    .unwrap_err();
                assert_eq!(
                    err,
                    NymOfflineSignersContractError::RecentlyCameOnline {
                        address: signer.clone()
                    }
                );

                tester.advance_day_of_blocks();
                let env = tester.env();
                let res = storage.propose_or_vote(tester.deps_mut(), env, proposer, signer);

                assert!(res.is_ok());
                Ok(())
            }

            #[test]
            fn returns_quorum_information() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut tester =
                    init_custom_contract_tester(10, init_with_quorum(Decimal::percent(30)));

                let voter1 = tester.random_group_member();
                let voter2 = tester.random_group_member();
                let voter3 = tester.random_group_member();
                let signer = tester.random_group_member();
                assert!([&voter1, &voter2, &voter3, &signer]
                    .iter()
                    .duplicates()
                    .next()
                    .is_none());

                let env = tester.env();
                assert!(!storage.propose_or_vote(
                    tester.deps_mut(),
                    env.clone(),
                    voter1,
                    signer.clone()
                )?);
                assert!(!storage.propose_or_vote(
                    tester.deps_mut(),
                    env.clone(),
                    voter2,
                    signer.clone()
                )?);
                assert!(storage.propose_or_vote(tester.deps_mut(), env, voter3, signer.clone())?);

                Ok(())
            }
        }

        #[cfg(test)]
        mod reset_offline_status {
            use super::*;
            use nym_contracts_common_testing::ChainOpts;

            #[test]
            fn signer_must_have_been_offline_for_threshold_period() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut tester = init_contract_tester();

                let threshold = storage.config.load(&tester)?.status_change_cooldown_secs;

                let signer = tester.random_group_member();
                tester.insert_offline_signer(&signer);

                // try to reset it immediately
                let env = tester.env();
                let err = storage
                    .reset_offline_status(tester.deps_mut(), env, signer.clone())
                    .unwrap_err();
                assert_eq!(
                    err,
                    NymOfflineSignersContractError::RecentlyCameOffline {
                        address: signer.clone()
                    }
                );

                // wait for the minimum period MINUS one second (so just barely out of it)
                tester.advance_time_by(threshold - 1);
                let env = tester.env();
                let err = storage
                    .reset_offline_status(tester.deps_mut(), env, signer.clone())
                    .unwrap_err();
                assert_eq!(
                    err,
                    NymOfflineSignersContractError::RecentlyCameOffline {
                        address: signer.clone()
                    }
                );

                // wait additional second (i.e. exactly minimum period)
                tester.advance_time_by(1);
                let env = tester.env();
                let res = storage.reset_offline_status(tester.deps_mut(), env, signer.clone());
                assert!(res.is_ok());

                // another instance, way beyond minimum value
                let another_signer = tester.random_group_member();
                tester.insert_offline_signer(&another_signer);
                tester.advance_time_by(10 * threshold);
                let env = tester.env();
                let res =
                    storage.reset_offline_status(tester.deps_mut(), env, another_signer.clone());
                assert!(res.is_ok());

                Ok(())
            }

            #[test]
            fn signer_must_be_actually_offline() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut tester = init_contract_tester();

                let signer = tester.random_group_member();

                tester.advance_day_of_blocks();
                let env = tester.env();
                let err = storage
                    .reset_offline_status(tester.deps_mut(), env, signer.clone())
                    .unwrap_err();
                assert_eq!(
                    err,
                    NymOfflineSignersContractError::NotOffline {
                        address: signer.clone()
                    }
                );

                // after marking it, it's fine now
                tester.insert_offline_signer(&signer);
                tester.advance_day_of_blocks();
                let env = tester.env();
                let res = storage.reset_offline_status(tester.deps_mut(), env, signer.clone());
                assert!(res.is_ok());

                Ok(())
            }

            #[test]
            fn clears_offline_status_and_updates_last_reset() -> anyhow::Result<()> {
                let storage = NymOfflineSignersStorage::new();
                let mut tester = init_contract_tester();

                let signer = tester.random_group_member();
                tester.insert_offline_signer(&signer);
                tester.advance_day_of_blocks();

                assert!(storage
                    .offline_signers
                    .addresses
                    .load(&tester)?
                    .contains(&signer));
                assert!(storage.offline_signers.information.has(&tester, &signer));
                assert!(storage.active_proposals.has(&tester, &signer));

                let env = tester.env();
                storage.reset_offline_status(tester.deps_mut(), env, signer.clone())?;

                assert!(!storage
                    .offline_signers
                    .addresses
                    .load(&tester)?
                    .contains(&signer));
                assert!(!storage.offline_signers.information.has(&tester, &signer));
                assert!(!storage.active_proposals.has(&tester, &signer));

                Ok(())
            }
        }
    }

    #[cfg(test)]
    mod offline_signers_storage {
        use super::*;
        use crate::testing::{init_contract_tester, OfflineSignersContractTesterExt};
        use cosmwasm_std::testing::mock_env;
        use nym_contracts_common_testing::{
            mock_dependencies, ChainOpts, ContractOpts, FullReader, RandExt,
        };

        fn mock_offline_signer_info() -> OfflineSignerInformation {
            OfflineSignerInformation {
                marked_offline_at: mock_env().block,
                associated_proposal: 123,
            }
        }

        #[test]
        fn initialisation() -> anyhow::Result<()> {
            let storage = OfflineSignersStorage::new();

            let mut empty_deps = mock_dependencies();
            assert!(storage
                .addresses
                .may_load(empty_deps.as_mut().storage)?
                .is_none());
            let mut tester = init_contract_tester();

            assert!(storage.addresses.may_load(tester.storage_mut())?.is_some());
            Ok(())
        }

        #[test]
        fn checking_for_signer_information() -> anyhow::Result<()> {
            let storage = OfflineSignersStorage::new();
            let mut tester = init_contract_tester();

            let random_signer = tester.random_group_member();

            // nothing initially
            assert!(!storage.has_signer_information(&tester, &random_signer));

            // after marking it offline it's there
            tester.insert_offline_signer(&random_signer);
            assert!(storage.has_signer_information(&tester, &random_signer));

            // and it's gone after the removal
            tester.advance_day_of_blocks();
            tester.reset_offline_status(&random_signer);
            assert!(!storage.has_signer_information(&tester, &random_signer));

            Ok(())
        }

        #[test]
        fn retrieving_signer_information() -> anyhow::Result<()> {
            let storage = OfflineSignersStorage::new();
            let mut tester = init_contract_tester();

            let random_signer = tester.random_group_member();

            // nothing initially
            assert!(storage
                .load_signer_information(&tester, &random_signer)?
                .is_none());

            // after marking it offline it's there
            let proposal_id = tester.insert_offline_signer(&random_signer);
            let loaded = storage
                .load_signer_information(&tester, &random_signer)?
                .unwrap();
            assert_eq!(loaded.associated_proposal, proposal_id);

            // and it's gone after the removal
            tester.advance_day_of_blocks();
            tester.reset_offline_status(&random_signer);
            assert!(storage
                .load_signer_information(&tester, &random_signer)?
                .is_none());

            Ok(())
        }

        #[test]
        fn insertion_puts_data_in_map_and_item() -> anyhow::Result<()> {
            let storage = OfflineSignersStorage::new();
            let mut tester = init_contract_tester();

            // initial
            let initial_env = tester.env();
            assert!(storage.addresses.load(&tester)?.is_empty());
            assert!(storage.information.all_values(&tester)?.is_empty());
            tester.next_block();

            for i in 0..10 {
                let random_signer = tester.generate_account();
                let env = tester.env();
                storage.insert_offline_signer_information(
                    tester.storage_mut(),
                    &env,
                    &random_signer,
                    &mock_offline_signer_info(),
                )?;
                tester.next_block();

                assert_eq!(storage.addresses.load(&tester)?.len(), i + 1);
                assert_eq!(storage.information.all_values(&tester)?.len(), i + 1);
            }

            // check snapshots
            for i in 0..10 {
                // add additional block as insertion happened at the beginning of the block
                let height = initial_env.block.height + i + 1;

                assert_eq!(
                    storage
                        .addresses
                        .may_load_at_height(&tester, height)?
                        .unwrap()
                        .len(),
                    i as usize
                );
            }

            Ok(())
        }

        #[test]
        fn removal_removes_data_from_map_and_item() -> anyhow::Result<()> {
            let storage = OfflineSignersStorage::new();
            let mut tester = init_contract_tester();

            let initial_env = tester.env();
            tester.next_block();

            let mut inserted = Vec::new();
            for _ in 0..10 {
                let random_signer = tester.generate_account();
                let env = tester.env();
                storage.insert_offline_signer_information(
                    tester.storage_mut(),
                    &env,
                    &random_signer,
                    &mock_offline_signer_info(),
                )?;
                tester.next_block();
                inserted.push(random_signer);
            }

            for signer in &inserted {
                // before is present in both
                let addresses = storage.addresses.load(&tester)?;
                assert!(addresses.contains(signer));
                assert!(storage.information.has(&tester, signer));

                let env = tester.env();
                storage.remove_offline_signer_information(tester.storage_mut(), &env, signer)?;
                tester.next_block();

                // after is gone
                let addresses = storage.addresses.load(&tester)?;
                assert!(!addresses.contains(signer));
                assert!(!storage.information.has(&tester, signer));
            }

            // check snapshots
            for i in 0..10 {
                let height = initial_env.block.height + i + 1 + inserted.len() as u64;

                assert_eq!(
                    inserted.len()
                        - storage
                            .addresses
                            .may_load_at_height(&tester, height)?
                            .unwrap()
                            .len(),
                    i as usize
                );
            }

            Ok(())
        }
    }
}
