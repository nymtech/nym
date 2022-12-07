// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::state::State;
use crate::coconut::error::CoconutError;

pub(crate) async fn public_key_submission(
    dkg_client: &DkgClient,
    state: &mut State,
) -> Result<(), CoconutError> {
    if state.node_index().is_some() {
        return Ok(());
    }

    let bte_key = bs58::encode(&state.dkg_keypair().public_key().to_bytes()).into_string();
    let index = if let Some(details) = dkg_client
        .get_self_registered_dealer_details()
        .await?
        .details
    {
        details.assigned_index
    } else {
        dkg_client
            .register_dealer(bte_key, state.announce_address().to_string())
            .await?
    };
    state.set_node_index(Some(index));
    info!("DKG: Using node index {}", index);

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::coconut::tests::DummyClient;
    use crate::coconut::KeyPair;
    use dkg::bte::keys::KeyPair as DkgKeyPair;
    use rand::rngs::OsRng;
    use std::str::FromStr;
    use url::Url;
    use validator_client::nymd::AccountId;

    const TEST_VALIDATOR_ADDRESS: &str = "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";

    #[tokio::test]
    #[ignore] // expensive test
    async fn submit_public_key() {
        let dkg_client = DkgClient::new(DummyClient::new(
            AccountId::from_str(TEST_VALIDATOR_ADDRESS).unwrap(),
        ));
        let mut state = State::new(
            Url::parse("localhost:8000").unwrap(),
            DkgKeyPair::new(&dkg::bte::setup(), OsRng),
            KeyPair::new(),
        );

        assert!(dkg_client
            .get_self_registered_dealer_details()
            .await
            .unwrap()
            .details
            .is_none());
        public_key_submission(&dkg_client, &mut state)
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
        public_key_submission(&dkg_client, &mut state)
            .await
            .unwrap();
        assert_eq!(state.node_index().unwrap(), client_idx);
    }
}
