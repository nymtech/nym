// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::state::State;
use crate::coconut::error::CoconutError;
use log::debug;
use nym_coconut_dkg_common::dealer::DealerType;

pub(crate) async fn public_key_submission(
    dkg_client: &DkgClient,
    state: &mut State,
    resharing: bool,
) -> Result<(), CoconutError> {
    if state.was_in_progress() {
        let own_address = dkg_client.get_address().await.as_ref().to_string();
        let is_initial_dealer = dkg_client
            .get_initial_dealers()
            .await?
            .map(|data| data.initial_dealers.iter().any(|d| *d == own_address))
            .unwrap_or(false);
        let reset_coconut_keypair = !resharing || !is_initial_dealer;
        debug!(
            "Resetting state, with coconut keypair reset: {}",
            reset_coconut_keypair
        );
        state.reset_persistent(reset_coconut_keypair).await;
    }
    if state.node_index().is_some() {
        debug!("Node index was set previously, nothing to do");
        return Ok(());
    }

    let bte_key = bs58::encode(&state.dkg_keypair().public_key().to_bytes()).into_string();
    let dealer_details = dkg_client.get_self_registered_dealer_details().await?;
    let index = if let Some(details) = dealer_details.details {
        if dealer_details.dealer_type == DealerType::Past {
            // If it was a dealer in a previous epoch, re-register it for this epoch
            debug!("Registering for the current DKG round, with keys from a previous epoch");
            dkg_client
                .register_dealer(bte_key, state.announce_address().to_string(), resharing)
                .await?;
        }
        details.assigned_index
    } else {
        debug!("Registering for the first time to be a dealer");
        // First time registration
        dkg_client
            .register_dealer(bte_key, state.announce_address().to_string(), resharing)
            .await?
    };
    state.set_node_index(Some(index));
    info!("DKG: Using node index {}", index);

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::coconut::dkg::state::PersistentState;
    use crate::coconut::tests::DummyClient;
    use crate::coconut::KeyPair;
    use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
    use nym_validator_client::nyxd::AccountId;
    use rand::rngs::OsRng;
    use std::path::PathBuf;
    use std::str::FromStr;
    use url::Url;

    const TEST_VALIDATOR_ADDRESS: &str = "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";

    #[tokio::test]
    #[ignore] // expensive test
    async fn submit_public_key() {
        let dkg_client = DkgClient::new(DummyClient::new(
            AccountId::from_str(TEST_VALIDATOR_ADDRESS).unwrap(),
        ));
        let mut state = State::new(
            PathBuf::default(),
            PersistentState::default(),
            Url::parse("localhost:8000").unwrap(),
            DkgKeyPair::new(&nym_dkg::bte::setup(), OsRng),
            KeyPair::new(),
        );

        assert!(dkg_client
            .get_self_registered_dealer_details()
            .await
            .unwrap()
            .details
            .is_none());
        public_key_submission(&dkg_client, &mut state, false)
            .await
            .unwrap();
        let client_idx = dkg_client
            .get_self_registered_dealer_details()
            .await
            .unwrap()
            .details
            .unwrap()
            .assigned_index;
        assert_eq!(state.node_index().unwrap(), client_idx);

        // keeps the same index from chain, not calling register_dealer again
        state.set_node_index(None);
        public_key_submission(&dkg_client, &mut state, false)
            .await
            .unwrap();
        assert_eq!(state.node_index().unwrap(), client_idx);
    }
}
