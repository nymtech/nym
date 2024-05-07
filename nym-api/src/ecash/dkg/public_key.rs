// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::dkg::controller::DkgController;
use crate::ecash::error::CoconutError;
use log::debug;
use nym_coconut_dkg_common::types::EpochId;
use rand::{CryptoRng, RngCore};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PublicKeySubmissionError {
    #[error(transparent)]
    CoconutError(#[from] CoconutError),
}

impl<R: RngCore + CryptoRng> DkgController<R> {
    /// First step of the DKG process during which the nym api will register for the key exchange
    /// by submitting its:
    /// - BTE public key (alongside the proof of discrete log)
    /// - ed25519 public key
    /// - announce address to be used by clients for obtaining credentials
    /// Upon successful registration, the node will receive a unique "NodeIndex"
    /// which is the x-coordinate of the to be derived keys.
    ///
    /// During this step any prior coconut keys will be invalidated, i.e. keys from the previous epoch
    /// won't be used for issuing new credentials.
    ///
    /// Furthermore, if the node experienced any failures during this step, a recovery will be attempted.
    pub(crate) async fn public_key_submission(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), PublicKeySubmissionError> {
        self.state.maybe_init_dkg_state(epoch_id);
        let registration_state = self.state.registration_state(epoch_id)?;

        // check if we have already submitted the key
        if registration_state.completed() {
            // the only way this could be a false positive is if the chain forked and blocks got reverted,
            // but I don't think we have to worry about that
            debug!("we have already submitted the keys for this epoch");
            return Ok(());
        }

        // if we have coconut keys available, it means we have already completed the DKG before (in previous epoch)
        // in which case, invalidate it so that it wouldn't be used for credential issuance
        self.state.invalidate_coconut_keypair();

        // FAILURE CASE:
        // check if we have already sent the registration transaction, but it timed out or got stuck in the mempool and
        // eventually got executed without us knowing about it
        // in that case we MUST recover the assigned index since we won't be allowed to register again
        let dealer_details = self.dkg_client.get_self_registered_dealer_details().await?;
        if dealer_details.dealer_type.is_current() {
            if let Some(details) = dealer_details.details {
                // the tx did actually go through
                self.state.registration_state_mut(epoch_id)?.assigned_index =
                    Some(details.assigned_index);
                info!("DKG: recovered node index: {}", details.assigned_index);
                return Ok(());
            }
        }

        // perform the full registration instead
        let bte_key = bs58::encode(&self.state.dkg_keypair().public_key().to_bytes()).into_string();
        let identity_key = self.state.identity_key().to_base58_string();
        let announce_address = self.state.announce_address().to_string();

        let assigned_index = self
            .dkg_client
            .register_dealer(bte_key, identity_key, announce_address, resharing)
            .await?;
        self.state.registration_state_mut(epoch_id)?.assigned_index = Some(assigned_index);
        info!("DKG: Using node index {assigned_index}");

        Ok(())
    }
}

// NOTE: the following tests currently do NOT cover all cases
// I've (@JS) only updated old, existing, tests. nothing more
#[cfg(test)]
pub(crate) mod tests {
    use crate::ecash::tests::fixtures;

    #[tokio::test]
    async fn submit_public_key() -> anyhow::Result<()> {
        let mut controller = fixtures::dkg_controller_fixture().await;
        let epoch = controller.dkg_client.get_current_epoch().await?.epoch_id;

        assert!(controller
            .dkg_client
            .get_self_registered_dealer_details()
            .await?
            .details
            .is_none());
        controller.public_key_submission(epoch, false).await?;
        let client_idx = controller
            .dkg_client
            .get_self_registered_dealer_details()
            .await?
            .details
            .unwrap()
            .assigned_index;
        assert_eq!(
            controller
                .state
                .registration_state(epoch)?
                .assigned_index
                .unwrap(),
            client_idx
        );

        // keeps the same index from chain, not calling register_dealer again
        controller
            .state
            .registration_state_mut(epoch)?
            .assigned_index = None;
        controller.public_key_submission(epoch, false).await?;
        assert_eq!(
            controller
                .state
                .registration_state(epoch)?
                .assigned_index
                .unwrap(),
            client_idx
        );

        Ok(())
    }
}
