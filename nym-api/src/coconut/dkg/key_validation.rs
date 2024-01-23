// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::api_routes::epoch_credentials;
use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::controller::DkgController;
use crate::coconut::dkg::state::State;
use crate::coconut::error::CoconutError;
use crate::coconut::helpers::accepted_vote_err;
use crate::coconut::state::BANDWIDTH_CREDENTIAL_PARAMS;
use cosmwasm_std::Addr;
use cw3::{ProposalResponse, Status};
use nym_coconut::tests::helpers::transpose_matrix;
use nym_coconut::{check_vk_pairing, Base58, VerificationKey};
use nym_coconut_dkg_common::types::EpochId;
use nym_coconut_dkg_common::verification_key::{owner_from_cosmos_msgs, ContractVKShare};
use nym_validator_client::nyxd::AccountId;
use rand::{CryptoRng, RngCore};
use std::collections::HashMap;

impl<R: RngCore + CryptoRng> DkgController<R> {
    fn filter_proposal(
        &self,
        dkg_contract: &AccountId,
        proposal: &ProposalResponse,
    ) -> Option<(Addr, u64)> {
        // make sure the proposal we're checking is:
        // - still open (not point in voting for anything that has already expired)
        // - was proposed by the DKG contract - so that we'd ignore anything from malicious dealers
        // - contains valid verification request (checked inside `owner_from_cosmos_msgs`)
        if proposal.status == Status::Open && proposal.proposer.as_str() == dkg_contract.as_ref() {
            if let Some(owner) = owner_from_cosmos_msgs(&proposal.msgs) {
                return Some((owner, proposal.id));
            }
        }
        None
    }

    async fn get_validation_proposals(&self) -> Result<HashMap<Addr, u64>, CoconutError> {
        let dkg_contract = self.dkg_client.dkg_contract_address().await?;

        // FUTURE OPTIMIZATION: don't query for ALL proposals. say if we're in epoch 50,
        // we don't care about expired proposals from epochs 0-49...
        // to do it, we'll need to have dkg contract store proposal ids,
        // which will require usage of submsgs and replies so that might be a future project
        let all_proposals = self.dkg_client.list_proposals().await?;

        let mut deduped_proposals = HashMap::new();

        // for each proposal, make sure it's a valid validation request;
        // if for some reason there exist multiple proposals from the same owner, choose the one
        // with the higher id
        for proposal in all_proposals {
            if let Some((owner, id)) = self.filter_proposal(&dkg_contract, &proposal) {
                if let Some(old_id) = deduped_proposals.get(&owner) {
                    if old_id < &id {
                        deduped_proposals.insert(owner, id);
                    }
                } else {
                    deduped_proposals.insert(owner, id);
                }
            }
        }

        // UNHANDLED EDGE CASE:
        // since currently proposals are **NOT** tied to epochs,
        // we might run into proposals from older epochs we don't have to vote on or might not even have data for
        Ok(deduped_proposals)
    }

    async fn verify_share(
        &self,
        epoch_id: EpochId,
        share: ContractVKShare,
    ) -> Result<(), CoconutError> {
        if share.verified {
            todo!()
        }

        let Some(receiver_index) = self
            .state
            .valid_epoch_receivers(epoch_id)?
            .iter()
            .position(|(addr, _)| addr == share.owner)
        else {
            todo!()
        };

        // EDGE CASE:
        // make sure the receiver index of this receiver/dealer is within the size of the derived keys

        // attempt to recover the underlying key from its bs58 representation
        let recovered_key = match VerificationKey::try_from_bs58(share.share) {
            Ok(key) => key,
            Err(err) => {
                warn!(
                    "failed to decode verification share from {}: {err}",
                    share.owner
                );
                todo!()
            }
        };

        let Some(self_derived) = self
            .state
            .key_derivation_state(epoch_id)?
            .derived_partials_for(receiver_index)
        else {
            todo!()
        };

        if !check_vk_pairing(&BANDWIDTH_CREDENTIAL_PARAMS, &self_derived, &recovered_key) {
            todo!()
        }

        Ok(())
    }

    pub(crate) async fn verification_key_validation(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), CoconutError> {
        let key_validation_state = self.state.key_validation_state(epoch_id)?;

        // check if we have already validated and voted for all keys
        if key_validation_state.completed() {
            // the only way this could be a false positive is if the chain forked and blocks got reverted,
            // but I don't think we have to worry about that
            debug!("we have already voted in all validation proposals");
            return Ok(());
        }

        // FAILURE CASE:
        // check if we have already verified the keys, but some voting txs either didn't get executed
        // or got executed without us knowing about it
        if !key_validation_state.votes.is_empty() {
            debug!(
                "we have already validated all keys for this epoch, but might have failed to vote"
            );
            // dkg_client.query_vote to check if our vote is there
            todo!()
        }

        let proposals = self.get_validation_proposals().await?;
        let vk_shares = self
            .dkg_client
            .get_verification_key_shares(epoch_id)
            .await?;

        for contract_share in vk_shares {
            // there's no point in checking anything if there doesn't exist an associated multisig proposal
            let Some(proposal_id) = proposals.get(&contract_share.owner) else {
                warn!(
                    "there does not seem to exist proposal for vk share from {}",
                    contract_share.owner
                );
                continue;
            };

            let vote = if let Err(err) = self.verify_share(epoch_id, contract_share).await {
                todo!();
                false
            } else {
                true
            };

            self.state
                .key_validation_state_mut(epoch_id)?
                .votes
                .insert(*proposal_id, vote);
        }

        // do vote
        for (&proposal, &vote) in &self.state.key_validation_state(epoch_id)?.votes {
            // FUTURE OPTIMIZATION: we could batch them in a single tx
            self.dkg_client
                .vote_verification_key_share(proposal, vote)
                .await?;
        }

        self.state.key_validation_state_mut(epoch_id)?.completed = true;

        // if self.state.voted_vks() {
        //     log::debug!("Already voted on the verification keys, nothing to do");
        //     return Ok(());
        // }
        //
        // let epoch_id = self.dkg_client.get_current_epoch().await?.epoch_id;
        // let vk_shares = self
        //     .dkg_client
        //     .get_verification_key_shares(epoch_id)
        //     .await?;
        // let proposal_ids = BTreeMap::from_iter(
        //     self.dkg_client
        //         .list_proposals()
        //         .await?
        //         .iter()
        //         .filter_map(validate_proposal),
        // );
        // let filtered_receivers_by_idx: Vec<_> = self
        //     .state
        //     .current_dealers_by_idx()
        //     .keys()
        //     .copied()
        //     .collect();
        // let recovered_partials: Vec<_> = self
        //     .state
        //     .recovered_vks()
        //     .iter()
        //     .map(|recovered_vk| recovered_vk.recovered_partials.clone())
        //     .collect();
        // let recovered_partials = transpose_matrix(recovered_partials);
        // let params = &BANDWIDTH_CREDENTIAL_PARAMS;
        // for contract_share in vk_shares {
        //     if let Some(proposal_id) = proposal_ids.get(&contract_share.owner).copied() {
        //         match VerificationKey::try_from_bs58(contract_share.share) {
        //             Ok(vk) => {
        //                 if let Some(idx) = filtered_receivers_by_idx
        //                     .iter()
        //                     .position(|node_index| contract_share.node_index == *node_index)
        //                 {
        //                     let ret = if !check_vk_pairing(params, &recovered_partials[idx], &vk) {
        //                         log::debug!(
        //                             "Voting NO to proposal {} because of failed VK pairing",
        //                             proposal_id
        //                         );
        //                         self.dkg_client
        //                             .vote_verification_key_share(proposal_id, false)
        //                             .await
        //                     } else {
        //                         log::debug!("Voting YES to proposal {}", proposal_id);
        //                         self.dkg_client
        //                             .vote_verification_key_share(proposal_id, true)
        //                             .await
        //                     };
        //                     accepted_vote_err(ret)?;
        //                 }
        //             }
        //             Err(_) => {
        //                 log::debug!(
        //                     "Voting NO to proposal {} because of failed base 58 deserialization",
        //                     proposal_id
        //                 );
        //                 let ret = self
        //                     .dkg_client
        //                     .vote_verification_key_share(proposal_id, false)
        //                     .await;
        //                 accepted_vote_err(ret)?;
        //             }
        //         }
        //     }
        // }
        // self.state.set_voted_vks();

        info!("DKG: validated all the other verification keys");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coconut::tests::helpers::{
        derive_keypairs, exchange_dealings, initialise_controllers, initialise_dkg,
        submit_public_keys,
    };

    #[tokio::test]
    #[ignore] // expensive test
    async fn validate_verification_key() -> anyhow::Result<()> {
        let validators = 4;

        let mut controllers = initialise_controllers(validators);
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_epoch.epoch_id;

        initialise_dkg(&mut controllers, false);
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;
        derive_keypairs(&mut controllers, false).await;

        for controller in controllers.iter_mut() {
            let res = controller.verification_key_validation(epoch, false).await;
            assert!(res.is_ok());

            assert!(controller.state.key_validation_state(epoch)?.completed);
        }

        let chain = controllers[0].chain_state.clone();
        let guard = chain.lock().unwrap();
        let proposals = &guard.proposals;
        assert_eq!(proposals.len(), validators);

        for proposal in proposals.values() {
            assert_eq!(Status::Passed, proposal.status)
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn validate_verification_key_malformed_share() {
        todo!()
        // let db = MockContractDb::new();
        // let mut clients_and_states = prepare_clients_and_states_with_submission(&db).await;
        //
        // db.verification_share_db
        //     .write()
        //     .unwrap()
        //     .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
        //     .and_modify(|share| share.share.push('x'));
        //
        // for controller in clients_and_states.iter_mut() {
        //     verification_key_validation(&controller.dkg_client, &mut controller.state, false)
        //         .await
        //         .unwrap();
        // }
        //
        // for (idx, controller) in clients_and_states.iter().enumerate() {
        //     let proposal = db
        //         .proposal_db
        //         .read()
        //         .unwrap()
        //         .get(&controller.state.proposal_id_value().unwrap())
        //         .unwrap()
        //         .clone();
        //     if idx == 0 {
        //         assert_eq!(proposal.status, Status::Rejected);
        //     } else {
        //         assert_eq!(proposal.status, Status::Passed);
        //     }
        // }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn validate_verification_key_unpaired_share() {
        todo!()
        // let db = MockContractDb::new();
        // let mut clients_and_states = prepare_clients_and_states_with_submission(&db).await;
        //
        // let second_share = db
        //     .verification_share_db
        //     .write()
        //     .unwrap()
        //     .get(TEST_VALIDATORS_ADDRESS[1])
        //     .unwrap()
        //     .share
        //     .clone();
        // db.verification_share_db
        //     .write()
        //     .unwrap()
        //     .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
        //     .and_modify(|share| share.share = second_share);
        //
        // for controller in clients_and_states.iter_mut() {
        //     verification_key_validation(&controller.dkg_client, &mut controller.state, false)
        //         .await
        //         .unwrap();
        // }
        //
        // for (idx, controller) in clients_and_states.iter().enumerate() {
        //     let proposal = db
        //         .proposal_db
        //         .read()
        //         .unwrap()
        //         .get(&controller.state.proposal_id_value().unwrap())
        //         .unwrap()
        //         .clone();
        //     if idx == 0 {
        //         assert_eq!(proposal.status, Status::Rejected);
        //     } else {
        //         assert_eq!(proposal.status, Status::Passed);
        //     }
        // }
    }
}
