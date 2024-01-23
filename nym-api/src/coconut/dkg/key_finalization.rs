// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::controller::DkgController;
use crate::coconut::dkg::state::{ConsistentState, State};
use crate::coconut::error::CoconutError;
use cw3::Status;
use nym_coconut_dkg_common::types::EpochId;
use rand::{CryptoRng, RngCore};

impl<R: RngCore + CryptoRng> DkgController<R> {
    pub(crate) async fn verification_key_finalization(
        &mut self,
        epoch_id: EpochId,
        _resharing: bool,
    ) -> Result<(), CoconutError> {
        let key_finalization_state = self.state.key_finalization_state(epoch_id)?;

        // check if we have already executed our own proposal
        if key_finalization_state.completed() {
            // the only way this could be a false positive is if the chain forked and blocks got reverted,
            // but I don't think we have to worry about that
            debug!("our key has already been verified");
            return Ok(());
        }

        let proposal_id = self.state.proposal_id(epoch_id)?;

        // check whether our key has already been verified with executed proposal,
        // either by us in previous iteration after a timeout
        // or by another party
        let status = self.dkg_client.get_proposal_status(proposal_id).await?;
        match status {
            Status::Pending | Status::Open => {
                warn!("our proposal ({proposal_id}) still hasn't received enough votes to get accepted");
                todo!();
            }
            Status::Rejected => {
                // TODO: technically there's nothing enforcing this...
                error!("our key verification proposal ({proposal_id}) has been rejected - we can't use our derived keys!");
                todo!()
            }
            Status::Passed => {
                self.dkg_client
                    .execute_verification_key_share(proposal_id)
                    .await?;
            }
            Status::Executed => {
                debug!("our dkg proposal has already been executed");
            }
        }

        self.state.key_finalization_state_mut(epoch_id)?.completed = true;
        info!("DKG: Finalized own verification key on chain");

        Ok(())
    }
}

// each dealer is responsible for executing its own proposals

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coconut::tests::helpers::{
        derive_keypairs, exchange_dealings, initialise_controllers, initialise_dkg,
        submit_public_keys, validate_keys,
    };

    #[tokio::test]
    #[ignore] // expensive test
    async fn finalize_verification_key() -> anyhow::Result<()> {
        let validators = 4;

        let mut controllers = initialise_controllers(validators);
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_epoch.epoch_id;

        initialise_dkg(&mut controllers, false);
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;
        derive_keypairs(&mut controllers, false).await;
        validate_keys(&mut controllers, false).await;

        for controller in controllers.iter_mut() {
            let res = controller.verification_key_finalization(epoch, false).await;
            assert!(res.is_ok());

            assert!(controller.state.key_finalization_state(epoch)?.completed);
        }

        let chain = controllers[0].chain_state.clone();
        let guard = chain.lock().unwrap();
        let proposals = &guard.proposals;
        assert_eq!(proposals.len(), validators);

        for proposal in proposals.values() {
            assert_eq!(Status::Executed, proposal.status)
        }

        Ok(())
    }
}
