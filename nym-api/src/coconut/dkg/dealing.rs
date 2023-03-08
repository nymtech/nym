// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::state::{ConsistentState, State};
use crate::coconut::error::CoconutError;
use nym_coconut_dkg_common::types::TOTAL_DEALINGS;
use nym_contracts_common::dealings::ContractSafeBytes;
use nym_dkg::bte::setup;
use nym_dkg::Dealing;
use rand::RngCore;
use std::collections::VecDeque;

pub(crate) async fn dealing_exchange(
    dkg_client: &DkgClient,
    state: &mut State,
    rng: impl RngCore + Clone,
    resharing: bool,
) -> Result<(), CoconutError> {
    if state.receiver_index().is_some() {
        return Ok(());
    }

    let dealers = dkg_client.get_current_dealers().await?;
    let threshold = dkg_client.get_current_epoch_threshold().await?;
    let initial_dealers = dkg_client
        .get_initial_dealers()
        .await?
        .map(|d| d.initial_dealers)
        .unwrap_or_default();
    let own_address = dkg_client.get_address().await.as_ref().to_string();
    state.set_dealers(dealers);
    state.set_threshold(threshold);
    let receivers = state.current_dealers_by_idx();
    let dealer_index = state.node_index_value()?;
    let receiver_index = receivers
        .keys()
        .position(|node_index| *node_index == dealer_index);

    let prior_resharing_secrets = if let Some(sk) = state.coconut_secret_key().await {
        // Double check that we are in resharing mode
        if resharing {
            let (x, mut scalars) = sk.into_raw();
            if scalars.len() + 1 != TOTAL_DEALINGS {
                return Err(CoconutError::CorruptedCoconutKeyPair);
            }
            // We can now erase the keypair from memory
            state.set_coconut_keypair(None).await;
            scalars.push(x);
            scalars
        } else {
            log::warn!("Coconut key hasn't been reset in memory. The state might be corrupt");
            vec![]
        }
    } else {
        vec![]
    };
    let mut prior_resharing_secrets = VecDeque::from(prior_resharing_secrets);
    if !resharing || initial_dealers.iter().any(|d| *d == own_address) {
        let params = setup();
        for _ in 0..TOTAL_DEALINGS {
            let (dealing, _) = Dealing::create(
                rng.clone(),
                &params,
                dealer_index,
                state.threshold()?,
                &receivers,
                prior_resharing_secrets.pop_front(),
            );
            dkg_client
                .submit_dealing(ContractSafeBytes::from(&dealing), resharing)
                .await?;
        }
    }

    info!("DKG: Finished submitting dealing");
    state.set_receiver_index(receiver_index);

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::coconut::dkg::complaints::ComplaintReason;
    use crate::coconut::dkg::state::PersistentState;
    use crate::coconut::tests::DummyClient;
    use crate::coconut::KeyPair;
    use cosmwasm_std::Addr;
    use nym_coconut_dkg_common::dealer::DealerDetails;
    use nym_coconut_dkg_common::types::InitialReplacementData;
    use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
    use nym_dkg::bte::{Params, PublicKeyWithProof};
    use nymcoconut::{ttp_keygen, Parameters};
    use rand::rngs::OsRng;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::sync::{Arc, RwLock};
    use url::Url;
    use validator_client::nyxd::AccountId;

    const TEST_VALIDATORS_ADDRESS: [&str; 4] = [
        "n1aq9kakfgwqcufr23lsv644apavcntrsqsk4yus",
        "n1s9l3xr4g0rglvk4yctktmck3h4eq0gp6z2e20v",
        "n19kl4py32vsk297dm93ezem992cdyzdy4zuc2x6",
        "n1jfrs6cmw9t7dv0x8cgny6geunzjh56n2s89fkv",
    ];

    fn insert_dealers(
        params: &Params,
        dealer_details_db: &Arc<RwLock<HashMap<String, DealerDetails>>>,
    ) -> Vec<DkgKeyPair> {
        let mut keypairs = vec![];
        for (idx, addr) in TEST_VALIDATORS_ADDRESS.iter().enumerate() {
            let keypair = DkgKeyPair::new(params, OsRng);
            let bte_public_key_with_proof =
                bs58::encode(&keypair.public_key().to_bytes()).into_string();
            keypairs.push(keypair);
            dealer_details_db.write().unwrap().insert(
                addr.to_string(),
                DealerDetails {
                    address: Addr::unchecked(*addr),
                    bte_public_key_with_proof,
                    announce_address: format!("localhost:80{}", idx),
                    assigned_index: (idx + 1) as u64,
                },
            );
        }
        keypairs
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn exchange_dealing() {
        let self_index = 2;
        let dealer_details_db = Arc::new(RwLock::new(HashMap::new()));
        let dealings_db = Arc::new(RwLock::new(HashMap::new()));
        let threshold_db = Arc::new(RwLock::new(Some(2)));
        let dkg_client = DkgClient::new(
            DummyClient::new(AccountId::from_str(TEST_VALIDATORS_ADDRESS[0]).unwrap())
                .with_dealer_details(&dealer_details_db)
                .with_dealings(&dealings_db)
                .with_threshold(&threshold_db),
        );
        let params = setup();
        let mut state = State::new(
            PathBuf::default(),
            PersistentState::default(),
            Url::parse("localhost:8000").unwrap(),
            DkgKeyPair::new(&params, OsRng),
            KeyPair::new(),
        );
        state.set_node_index(Some(self_index));
        let keypairs = insert_dealers(&params, &dealer_details_db);

        dealing_exchange(&dkg_client, &mut state, OsRng, false)
            .await
            .unwrap();

        assert_eq!(
            state.current_dealers_by_idx().values().collect::<Vec<_>>(),
            keypairs
                .iter()
                .map(|k| k.public_key().public_key())
                .collect::<Vec<_>>()
        );
        assert_eq!(state.threshold().unwrap(), 2);
        assert_eq!(state.receiver_index().unwrap(), 1);
        let dealings = dealings_db
            .read()
            .unwrap()
            .get(TEST_VALIDATORS_ADDRESS[0])
            .unwrap()
            .clone();
        assert_eq!(dealings.len(), TOTAL_DEALINGS);

        dealing_exchange(&dkg_client, &mut state, OsRng, false)
            .await
            .unwrap();
        let new_dealings = dealings_db
            .read()
            .unwrap()
            .get(TEST_VALIDATORS_ADDRESS[0])
            .unwrap()
            .clone();
        assert_eq!(dealings, new_dealings);
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn invalid_bte_proof_dealing_posted() {
        let self_index = 2;
        let dealer_details_db = Arc::new(RwLock::new(HashMap::new()));
        let dealings_db = Arc::new(RwLock::new(HashMap::new()));
        let threshold_db = Arc::new(RwLock::new(Some(2)));
        let dkg_client = DkgClient::new(
            DummyClient::new(AccountId::from_str(TEST_VALIDATORS_ADDRESS[0]).unwrap())
                .with_dealer_details(&dealer_details_db)
                .with_dealings(&dealings_db)
                .with_threshold(&threshold_db),
        );
        let params = setup();
        let mut state = State::new(
            PathBuf::default(),
            PersistentState::default(),
            Url::parse("localhost:8000").unwrap(),
            DkgKeyPair::new(&params, OsRng),
            KeyPair::new(),
        );
        state.set_node_index(Some(self_index));
        insert_dealers(&params, &dealer_details_db);

        dealer_details_db
            .write()
            .unwrap()
            .entry(TEST_VALIDATORS_ADDRESS[1].to_string())
            .and_modify(|details| {
                let mut bytes = bs58::decode(details.bte_public_key_with_proof.clone())
                    .into_vec()
                    .unwrap();
                // Find another value for last byte that still deserializes to a public key with proof
                let initial_byte = *bytes.last_mut().unwrap();
                loop {
                    let last_byte = bytes.last_mut().unwrap();
                    let (ret, _) = last_byte.overflowing_add(1);
                    *last_byte = ret;
                    // stop when we find that value, or if we do a full round trip of u8 values
                    // and can't find one, in which case this test is invalid
                    if PublicKeyWithProof::try_from_bytes(&bytes).is_ok() || ret == initial_byte {
                        break;
                    }
                }
                details.bte_public_key_with_proof = bs58::encode(&bytes).into_string();
            });

        dealing_exchange(&dkg_client, &mut state, OsRng, false)
            .await
            .unwrap();
        assert_eq!(
            *state
                .all_dealers()
                .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[1]))
                .unwrap()
                .as_ref()
                .unwrap_err(),
            ComplaintReason::InvalidBTEPublicKey
        );
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn resharing_exchange_dealing() {
        let self_index = 2;
        let dealer_details_db = Arc::new(RwLock::new(HashMap::new()));
        let dealings_db = Arc::new(RwLock::new(HashMap::new()));
        let threshold_db = Arc::new(RwLock::new(Some(3)));
        let initial_dealers_db = Arc::new(RwLock::new(Some(InitialReplacementData {
            initial_dealers: vec![Addr::unchecked(TEST_VALIDATORS_ADDRESS[0])],
            initial_height: Some(100),
        })));
        let dkg_client = DkgClient::new(
            DummyClient::new(
                AccountId::from_str("n1vxkywf9g4cg0k2dehanzwzz64jw782qm0kuynf").unwrap(),
            )
            .with_dealer_details(&dealer_details_db)
            .with_dealings(&dealings_db)
            .with_threshold(&threshold_db)
            .with_initial_dealers_db(&initial_dealers_db),
        );
        let params = setup();
        let mut keys = ttp_keygen(&Parameters::new(4).unwrap(), 3, 4).unwrap();
        let coconut_keypair = KeyPair::new();
        coconut_keypair.set(Some(keys.pop().unwrap())).await;

        let mut state = State::new(
            PathBuf::default(),
            PersistentState::default(),
            Url::parse("localhost:8000").unwrap(),
            DkgKeyPair::new(&params, OsRng),
            coconut_keypair.clone(),
        );
        state.set_node_index(Some(self_index));
        let keypairs = insert_dealers(&params, &dealer_details_db);

        dealing_exchange(&dkg_client, &mut state, OsRng, true)
            .await
            .unwrap();

        assert_eq!(
            state.current_dealers_by_idx().values().collect::<Vec<_>>(),
            keypairs
                .iter()
                .map(|k| k.public_key().public_key())
                .collect::<Vec<_>>()
        );
        assert_eq!(state.threshold().unwrap(), 3);
        assert_eq!(state.receiver_index().unwrap(), 1);
        let addr = dkg_client.get_address().await;
        assert!(dealings_db.read().unwrap().get(addr.as_ref()).is_none());

        let mut state = State::new(
            PathBuf::default(),
            PersistentState::default(),
            Url::parse("localhost:8000").unwrap(),
            DkgKeyPair::new(&params, OsRng),
            coconut_keypair,
        );
        state.set_node_index(Some(self_index));
        // Use a client that is in the initial dealers set
        let dkg_client = DkgClient::new(
            DummyClient::new(AccountId::from_str(TEST_VALIDATORS_ADDRESS[0]).unwrap())
                .with_dealer_details(&dealer_details_db)
                .with_dealings(&dealings_db)
                .with_threshold(&threshold_db)
                .with_initial_dealers_db(&initial_dealers_db),
        );

        dealing_exchange(&dkg_client, &mut state, OsRng, true)
            .await
            .unwrap();

        let dealings = dealings_db
            .read()
            .unwrap()
            .get(TEST_VALIDATORS_ADDRESS[0])
            .unwrap()
            .clone();
        assert_eq!(dealings.len(), TOTAL_DEALINGS);
    }
}
