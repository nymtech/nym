// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::dkg::controller::keys::archive_coconut_keypair;
use crate::coconut::dkg::controller::DkgController;
use crate::coconut::error::CoconutError;
use log::debug;
use nym_coconut_dkg_common::types::EpochId;
use rand::{CryptoRng, RngCore};

impl<R: RngCore + CryptoRng> DkgController<R> {
    pub(crate) async fn public_key_submission(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), CoconutError> {
        self.state.maybe_init_dkg_state(epoch_id);
        let registration_state = self.state.registration_state(epoch_id)?;

        // check if we have already submitted the key
        if registration_state.completed() {
            // the only way this could be a false positive is if the chain forked and blocks got reverted,
            // but I don't think we have to worry about that
            debug!("we have already submitted the keys for this epoch");
            return Ok(());
        }

        // // if we have coconut keys available, it means we have already completed the DKG before (in previous epoch)
        // // in which case, archive and reset those keys
        // if let Some(old_keypair) = self.state.take_coconut_keypair().await {
        //     debug!("resetting and archiving old coconut keypair");
        //     if let Err(source) =
        //         archive_coconut_keypair(&self.coconut_key_path, old_keypair.issued_for_epoch)
        //     {
        //         return Err(CoconutError::KeyArchiveFailure {
        //             epoch_id,
        //             path: self.coconut_key_path.clone(),
        //             source,
        //         });
        //     }
        // }

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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::coconut::dkg::client::DkgClient;
    use crate::coconut::dkg::state::{PersistentState, State};
    use crate::coconut::tests::DummyClient;
    use crate::coconut::KeyPair;
    use nym_crypto::asymmetric::identity;
    use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
    use nym_validator_client::nyxd::AccountId;
    use rand::rngs::OsRng;
    use rand_07::thread_rng;
    use std::path::PathBuf;
    use std::str::FromStr;
    use url::Url;

    const TEST_VALIDATOR_ADDRESS: &str = "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";

    #[tokio::test]
    async fn submit_public_key() -> anyhow::Result<()> {
        let dkg_client = DkgClient::new(DummyClient::new(
            AccountId::from_str(TEST_VALIDATOR_ADDRESS).unwrap(),
        ));
        let identity_keypair = identity::KeyPair::new(&mut thread_rng());
        let state = State::new(
            PathBuf::default(),
            PersistentState::default(),
            Url::parse("localhost:8000").unwrap(),
            DkgKeyPair::new(&nym_dkg::bte::setup(), OsRng),
            *identity_keypair.public_key(),
            KeyPair::new(),
        );
        let epoch = dkg_client.get_current_epoch().await.unwrap().epoch_id;
        let mut controller = DkgController::test_mock(dkg_client, state);

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
