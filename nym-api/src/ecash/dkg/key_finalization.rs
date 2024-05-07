// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::dkg::controller::DkgController;
use crate::ecash::error::CoconutError;
use cw3::Status;
use nym_coconut_dkg_common::types::EpochId;
use rand::{CryptoRng, RngCore};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyFinalizationError {
    #[error(transparent)]
    CoconutError(#[from] CoconutError),

    #[error("our proposal for key verification is still open (or is pending) (proposal id: {proposal_id}) ")]
    UnresolvedProposal { proposal_id: u64 },

    #[error("our proposal for key verification has been rejected (proposal id: {proposal_id})")]
    RejectedProposal { proposal_id: u64 },

    #[error("can't complete key finalization without key validation")]
    IncompleteKeyValidation,
}

impl<R: RngCore + CryptoRng> DkgController<R> {
    pub(crate) async fn verification_key_finalization(
        &mut self,
        epoch_id: EpochId,
    ) -> Result<(), KeyFinalizationError> {
        let key_finalization_state = self.state.key_finalization_state(epoch_id)?;

        // check if we have already executed our own proposal
        if key_finalization_state.completed() {
            // the only way this could be a false positive is if the chain forked and blocks got reverted,
            // but I don't think we have to worry about that
            debug!("our key has already been verified");
            return Ok(());
        }

        if !self.state.key_validation_state(epoch_id)?.completed {
            return Err(KeyFinalizationError::IncompleteKeyValidation);
        }

        let proposal_id = self.state.proposal_id(epoch_id)?;

        // check whether our key has already been verified with executed proposal,
        // either by us in previous iteration after a timeout
        // or by another party
        let status = self.dkg_client.get_proposal_status(proposal_id).await?;
        match status {
            // if the proposal hasn't been resolved, there's not much we can do but wait and pray
            Status::Pending | Status::Open => {
                // 'theoretically' it's possible that more votes are going to come in, but it's very unlikely
                warn!("our proposal ({proposal_id}) still hasn't received enough votes to get accepted");
                return Err(KeyFinalizationError::UnresolvedProposal { proposal_id });
            }
            // if the proposal has been rejected, there's nothing we can do, we failed the DKG
            Status::Rejected => {
                // technically there's nothing enforcing this, so as long as our keys have been properly generated
                // (even though they've been rejected by other parties), they could still issue [cryptographically] valid credentials
                error!("our key verification proposal ({proposal_id}) has been rejected - we can't use our derived keys!");
                self.state.key_finalization_state_mut(epoch_id)?.completed = true;
                return Err(KeyFinalizationError::RejectedProposal { proposal_id });
            }
            // if the proposal has passed, execute it to finalize our key
            Status::Passed => {
                self.dkg_client
                    .execute_verification_key_share(proposal_id)
                    .await?;
            }
            // if they proposal has already been executed, we're done!
            Status::Executed => {
                // generally each dealer is responsible for executing its own proposals,
                // but technically there's nothing preventing other dealers from executing them
                debug!("our dkg proposal has already been executed");
            }
        }

        self.state.key_finalization_state_mut(epoch_id)?.completed = true;
        self.state.validate_coconut_keypair();
        info!("DKG: Finalized own verification key on chain");

        Ok(())
    }
}

// NOTE: the following tests currently do NOT cover all cases
// I've (@JS) only updated old, existing, tests. nothing more
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecash::tests::helpers::{
        derive_keypairs, exchange_dealings, initialise_controllers, initialise_dkg,
        submit_public_keys, validate_keys,
    };

    #[tokio::test]
    #[ignore] // expensive test
    async fn finalize_verification_key() -> anyhow::Result<()> {
        let validators = 4;

        let mut controllers = initialise_controllers(validators).await;
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_contract.epoch.epoch_id;

        initialise_dkg(&mut controllers, false).await;
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;
        derive_keypairs(&mut controllers, false).await;
        validate_keys(&mut controllers, false).await;

        for controller in controllers.iter_mut() {
            let res = controller.verification_key_finalization(epoch).await;
            assert!(res.is_ok());

            assert!(controller.state.key_finalization_state(epoch)?.completed);
        }

        let chain = controllers[0].chain_state.clone();
        let guard = chain.lock().unwrap();
        let proposals = &guard.multisig_contract.proposals;
        assert_eq!(proposals.len(), validators);

        for proposal in proposals.values() {
            assert_eq!(Status::Executed, proposal.status)
        }

        Ok(())
    }
}
