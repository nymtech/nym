// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::dkg::controller::DkgController;
use crate::ecash::error::CoconutError;
use cosmwasm_std::Addr;
use cw3::Vote;
use nym_coconut_dkg_common::types::EpochId;
use nym_coconut_dkg_common::verification_key::ContractVKShare;
use nym_compact_ecash::{
    ecash_group_parameters, utils::check_vk_pairing, Base58, VerificationKeyAuth,
};
use rand::{CryptoRng, RngCore};
use std::collections::HashMap;
use thiserror::Error;

fn vote_matches(voted_yes: bool, chain_vote: Vote) -> bool {
    if voted_yes && chain_vote == Vote::Yes {
        true
    } else {
        !voted_yes && chain_vote == Vote::No
    }
}

#[derive(Debug, Error)]
pub enum KeyValidationError {
    #[error(transparent)]
    CoconutError(#[from] CoconutError),

    #[error("can't complete key validation without key derivation")]
    IncompleteKeyDerivation,
}

#[derive(Debug, Error)]
pub enum ShareRejectionReason {
    #[error("{owner} does not appear to be present in the list of receivers for epoch {epoch_id}")]
    NotAReceiver { epoch_id: EpochId, owner: Addr },

    #[error("the share from {owner} for epoch {epoch_id} already appears as verified on chain!")]
    AlreadyVerifiedOnChain { epoch_id: EpochId, owner: Addr },

    #[error(
        "the share from {owner} for epoch {epoch_id} does not use valid base58 encoding: {source}"
    )]
    MalformedKeyEncoding {
        epoch_id: EpochId,
        owner: Addr,
        #[source]
        source: nym_compact_ecash::error::CompactEcashError,
    },

    #[error("did not derive partial keys for {owner} at index {receiver_index} for epoch {epoch_id} during the dealings exchange")]
    MissingDerivedPartialKey {
        epoch_id: EpochId,
        owner: Addr,
        receiver_index: usize,
    },

    #[error("the provided keys {owner} at index {receiver_index} for epoch {epoch_id} either did not match the partial keys derived during the dealings exchange or failed the local bilinear pairing consistency check")]
    InconsistentKeys {
        epoch_id: EpochId,
        owner: Addr,
        receiver_index: usize,
    },
}

impl<R: RngCore + CryptoRng> DkgController<R> {
    async fn verify_share(
        &self,
        epoch_id: EpochId,
        share: ContractVKShare,
    ) -> Result<(Option<bool>, Option<ShareRejectionReason>), KeyValidationError> {
        fn reject(
            reason: ShareRejectionReason,
        ) -> Result<(Option<bool>, Option<ShareRejectionReason>), KeyValidationError> {
            Ok((Some(false), Some(reason)))
        }

        let owner = share.owner;

        if share.verified {
            error!("the share from {owner} has already been validated on chain - this should be impossible unless this machine is running seriously behind");
            let reason = ShareRejectionReason::AlreadyVerifiedOnChain { epoch_id, owner };
            // explicitly return 'None' for the vote as we don't have to (nor even should) vote for this share
            return Ok((None, Some(reason)));
        }

        // get the receiver index [of the dealings] for this participant
        let Some(receiver_index) = self
            .state
            .valid_epoch_receivers(epoch_id)?
            .iter()
            .position(|(addr, _)| addr == owner)
        else {
            return reject(ShareRejectionReason::NotAReceiver { epoch_id, owner });
        };

        // attempt to recover the underlying key from its bs58 representation
        let recovered_key = match VerificationKeyAuth::try_from_bs58(share.share) {
            Ok(key) => key,
            Err(source) => {
                return reject(ShareRejectionReason::MalformedKeyEncoding {
                    epoch_id,
                    owner,
                    source,
                });
            }
        };

        // retrieve the key we have recovered ourselves during the dealings exchange
        let Some(self_derived) = self
            .state
            .key_derivation_state(epoch_id)?
            .derived_partials_for(receiver_index)
        else {
            return reject(ShareRejectionReason::MissingDerivedPartialKey {
                epoch_id,
                owner,
                receiver_index,
            });
        };

        if !check_vk_pairing(ecash_group_parameters(), &self_derived, &recovered_key) {
            return reject(ShareRejectionReason::InconsistentKeys {
                epoch_id,
                owner,
                receiver_index,
            });
        }

        // all is good -> accept the keys!
        Ok((Some(true), None))
    }

    async fn generate_votes(
        &self,
        epoch_id: EpochId,
    ) -> Result<HashMap<u64, bool>, KeyValidationError> {
        let proposals = self.get_validation_proposals().await?;
        let vk_shares = self
            .dkg_client
            .get_verification_key_shares(epoch_id)
            .await?;

        let mut votes = HashMap::new();
        for contract_share in vk_shares {
            let owner = contract_share.owner.clone();
            debug!("verifying vk share from {owner}");

            // there's no point in checking anything if there doesn't exist an associated multisig proposal
            let Some(proposal_id) = proposals.get(owner.as_ref()) else {
                warn!("there does not seem to exist proposal for share validation from {owner}");
                continue;
            };

            // if this is our share, obviously vote for yes without spending time on verification
            if owner.as_ref() == self.dkg_client.get_address().await.as_ref() {
                votes.insert(*proposal_id, true);
                continue;
            }

            let (vote, rejection_reason) = self.verify_share(epoch_id, contract_share).await?;
            if let Some(vote) = vote {
                votes.insert(*proposal_id, vote);
            }
            if let Some(rejection_reason) = rejection_reason {
                warn!("rejecting share from {owner} (proposal: {proposal_id}): {rejection_reason}");
            }
        }

        Ok(votes)
    }

    async fn resubmit_validation_votes(&self, epoch_id: EpochId) -> Result<(), KeyValidationError> {
        let key_validation_state = self.state.key_validation_state(epoch_id)?;

        for (&proposal, &vote) in &key_validation_state.votes {
            // check whether we might have already voted on this particular proposal
            // (the vote might have gotten stuck in the mempool)
            let chain_vote = self.dkg_client.get_vote(proposal).await?;
            if let Some(chain_vote) = chain_vote.vote {
                warn!("we have already voted for proposal {proposal} before - we probably crashed or the chain timed out!");

                // that's an extremely weird behaviour -> perhaps the user voted manually outside the nym-api,
                // but we can't do anything about it
                if !vote_matches(vote, chain_vote.vote) {
                    error!("our vote for proposal {proposal} doesn't match the on-chain data! We decided to vote '{vote}' but the chain has {:?}", chain_vote.vote);
                }
                continue;
            }
            warn!("we have already decided on the vote status for proposal {proposal} before (vote: {vote}), but failed to submit it");
            self.dkg_client
                .vote_verification_key_share(proposal, vote)
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn verification_key_validation(
        &mut self,
        epoch_id: EpochId,
    ) -> Result<(), KeyValidationError> {
        let key_validation_state = self.state.key_validation_state(epoch_id)?;

        // check if we have already validated and voted for all keys
        if key_validation_state.completed() {
            // the only way this could be a false positive is if the chain forked and blocks got reverted,
            // but I don't think we have to worry about that
            debug!("we have already voted in all validation proposals");
            return Ok(());
        }

        if !self
            .state
            .key_derivation_state(epoch_id)?
            .completed_with_success()
        {
            return Err(KeyValidationError::IncompleteKeyDerivation);
        }

        // FAILURE CASE:
        // check if we have already verified the keys, but some voting txs either didn't get executed
        // or got executed without us knowing about it
        if !key_validation_state.votes.is_empty() {
            debug!(
                "we have already validated all keys for this epoch, but might have failed to vote"
            );
            self.resubmit_validation_votes(epoch_id).await?;

            // if we managed to resubmit the votes (i.e. we didn't return an error)
            // it means the state is complete now
            info!("DKG: resubmitted previously generated votes - finished key validation");
            self.state.key_validation_state_mut(epoch_id)?.completed = true;
            return Ok(());
        }

        let votes = self.generate_votes(epoch_id).await?;
        self.state
            .key_validation_state_mut(epoch_id)?
            .votes
            .clone_from(&votes);

        // send the votes
        for (proposal, vote) in votes {
            // FUTURE OPTIMIZATION: we could batch them in a single tx
            self.dkg_client
                .vote_verification_key_share(proposal, vote)
                .await?;
        }

        self.state.key_validation_state_mut(epoch_id)?.completed = true;

        info!("DKG: validated all the other verification keys");
        Ok(())
    }
}

// NOTE: the following tests currently do NOT cover all cases
// I've (@JS) only updated old, existing, tests. nothing more
#[cfg(test)]
mod tests {
    use crate::ecash::tests::helpers::{
        derive_keypairs, exchange_dealings, initialise_controllers, initialise_dkg,
        submit_public_keys,
    };
    use cw3::Status;
    use nym_coconut_dkg_common::verification_key::owner_from_cosmos_msgs;

    #[tokio::test]
    #[ignore] // expensive test
    async fn validate_verification_key() -> anyhow::Result<()> {
        let validators = 4;

        let mut controllers = initialise_controllers(validators).await;
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_contract.epoch.epoch_id;

        initialise_dkg(&mut controllers, false).await;
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;
        derive_keypairs(&mut controllers, false).await;

        for controller in controllers.iter_mut() {
            let res = controller.verification_key_validation(epoch).await;
            assert!(res.is_ok());

            assert!(controller.state.key_validation_state(epoch)?.completed);
        }

        let guard = chain.lock().unwrap();
        let proposals = &guard.multisig_contract.proposals;
        assert_eq!(proposals.len(), validators);

        for proposal in proposals.values() {
            assert_eq!(Status::Passed, proposal.status)
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn validate_verification_key_malformed_share() -> anyhow::Result<()> {
        let validators = 4;

        let mut controllers = initialise_controllers(validators).await;
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_contract.epoch.epoch_id;

        initialise_dkg(&mut controllers, false).await;
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;
        derive_keypairs(&mut controllers, false).await;

        let first_dealer = controllers[0].dkg_client.get_address().await;

        {
            let mut guard = chain.lock().unwrap();
            let shares = guard
                .dkg_contract
                .verification_shares
                .get_mut(&epoch)
                .unwrap();
            let share = shares.get_mut(first_dealer.as_ref()).unwrap();
            // mess up the share
            share.share.push('x');
        }

        for controller in controllers.iter_mut() {
            let res = controller.verification_key_validation(epoch).await;
            assert!(res.is_ok());

            assert!(controller.state.key_validation_state(epoch)?.completed);
        }

        let guard = chain.lock().unwrap();
        let proposals = &guard.multisig_contract.proposals;
        assert_eq!(proposals.len(), validators);

        // the proposal from the first dealer would have gotten rejected
        for proposal in proposals.values() {
            let addr = owner_from_cosmos_msgs(&proposal.msgs).unwrap();
            if addr.as_str() == first_dealer.as_ref() {
                assert_eq!(Status::Rejected, proposal.status)
            } else {
                assert_eq!(Status::Passed, proposal.status)
            }
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn validate_verification_key_unpaired_share() -> anyhow::Result<()> {
        let validators = 2;

        let mut controllers = initialise_controllers(validators).await;
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_contract.epoch.epoch_id;

        initialise_dkg(&mut controllers, false).await;
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;
        derive_keypairs(&mut controllers, false).await;

        let first_dealer = controllers[0].dkg_client.get_address().await;
        let second_dealer = controllers[1].dkg_client.get_address().await;

        {
            let mut guard = chain.lock().unwrap();
            let shares = guard
                .dkg_contract
                .verification_shares
                .get_mut(&epoch)
                .unwrap();
            let second_share = shares.get(second_dealer.as_ref()).unwrap().clone();

            let share = shares.get_mut(first_dealer.as_ref()).unwrap();
            // mess up the share
            share.share = second_share.share;
        }

        for controller in controllers.iter_mut() {
            let res = controller.verification_key_validation(epoch).await;
            assert!(res.is_ok());

            assert!(controller.state.key_validation_state(epoch)?.completed);
        }

        let guard = chain.lock().unwrap();
        let proposals = &guard.multisig_contract.proposals;
        assert_eq!(proposals.len(), validators);

        // the proposal from the first dealer would have gotten rejected
        for proposal in proposals.values() {
            let addr = owner_from_cosmos_msgs(&proposal.msgs).unwrap();
            if addr.as_str() == first_dealer.as_ref() {
                assert_eq!(Status::Rejected, proposal.status)
            } else {
                assert_eq!(Status::Passed, proposal.status)
            }
        }

        Ok(())
    }
}
