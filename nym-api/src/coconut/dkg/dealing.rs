// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::dkg;
use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::complaints::ComplaintReason;
use crate::coconut::dkg::controller::keys::archive_coconut_keypair;
use crate::coconut::dkg::controller::DkgController;
use crate::coconut::dkg::state::{ConsistentState, ParticipantState, State};
use crate::coconut::error::CoconutError;
use crate::coconut::keys::KeyPairWithEpoch;
use log::debug;
use nym_coconut_dkg_common::types::{
    ContractDealing, DealingIndex, EpochId, PartialContractDealing,
};
use nym_dkg::bte::{PublicKey, PublicKeyWithProof};
use nym_dkg::{Dealing, NodeIndex, Scalar, Threshold};
use rand::{CryptoRng, RngCore};
use rocket::form::validate::Len;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt::{Debug, Formatter};
use zeroize::Zeroize;

enum DealingGeneration {
    Fresh { number: u32 },
    Resharing { prior_secrets: Vec<Scalar> },
}

impl Debug for DealingGeneration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DealingGeneration::Fresh { number } => f
                .debug_struct("DealingGeneration::Fresh")
                .field("number", number)
                .finish(),
            DealingGeneration::Resharing { prior_secrets } => f
                .debug_struct("DealingGeneration::Resharing")
                .field("number", &prior_secrets.len())
                .finish(),
        }
    }
}

impl<R: RngCore + CryptoRng> DkgController<R> {
    async fn generate_dealings(
        &mut self,
        epoch_id: EpochId,
        spec: DealingGeneration,
    ) -> Result<HashMap<DealingIndex, Dealing>, CoconutError> {
        let threshold = self.dkg_client.get_current_epoch_threshold().await?.ok_or(
            CoconutError::UnrecoverableState {
                reason: String::from("Threshold should have been set"),
            },
        )?;

        let dealer_index = self
            .state
            .registration_state(epoch_id)?
            .assigned_index
            .ok_or(CoconutError::UnrecoverableState {
                reason: String::from("Node index should have been set"),
            })?;

        // in our case every dealer is also a receiver

        // ASSUMPTION: all dealers see the same contract data, i.e. if one fails to decode and verify the receiver's key,
        // all of them will
        let filtered_receivers: BTreeMap<_, _> = self
            .state
            .dkg_state(epoch_id)?
            .dealers
            .iter()
            .filter_map(|(index, dealer)| match &dealer.state {
                ParticipantState::Invalid(_) => None,
                ParticipantState::VerifiedKey(key) => Some((*index, *key.public_key())),
            })
            .collect();

        let dbg_receivers = filtered_receivers.keys().collect::<Vec<_>>();
        debug!("generating dealings with threshold {threshold} for receivers: {dbg_receivers:?} with the following spec: {spec:?}. Our index is {dealer_index}");

        let mut dealings = HashMap::new();
        match spec {
            DealingGeneration::Fresh { number } => {
                for i in 0..number {
                    let dealing = Dealing::create(
                        &mut self.rng,
                        dkg::params(),
                        dealer_index,
                        threshold,
                        &filtered_receivers,
                        None,
                    );
                    dealings.insert(i as DealingIndex, dealing.0);
                }
            }
            DealingGeneration::Resharing { prior_secrets } => {
                for (i, secret) in prior_secrets.into_iter().enumerate() {
                    let dealing = Dealing::create(
                        &mut self.rng,
                        dkg::params(),
                        dealer_index,
                        threshold,
                        &filtered_receivers,
                        Some(secret),
                    );
                    dealings.insert(i as DealingIndex, dealing.0);
                }
            }
        }

        // update the state with the dealing information
        self.state
            .dealing_exchange_state_mut(epoch_id)?
            .generated_dealings = dealings.clone();

        Ok(dealings)
    }

    async fn generate_fresh_dealings(
        &mut self,
        epoch_id: EpochId,
        number: u32,
    ) -> Result<HashMap<DealingIndex, Dealing>, CoconutError> {
        self.generate_dealings(epoch_id, DealingGeneration::Fresh { number })
            .await
    }

    async fn generate_resharing_dealings(
        &mut self,
        epoch_id: EpochId,
        prior_secrets: Vec<Scalar>,
    ) -> Result<HashMap<DealingIndex, Dealing>, CoconutError> {
        self.generate_dealings(epoch_id, DealingGeneration::Resharing { prior_secrets })
            .await
    }

    async fn resubmit_pregenerated_dealings(
        &self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), CoconutError> {
        let dealing_state = self.state.dealing_exchange_state(epoch_id)?;

        for (dealing_index, dealing) in &dealing_state.generated_dealings {
            // check which dealing is actually present on the chain (some might have gotten stuck in the mempool for quite a while)
            let dealing_submitted = self
                .dkg_client
                .get_dealing_status(epoch_id, *dealing_index)
                .await?;

            if dealing_submitted {
                warn!("we have already submitted dealing {dealing_index} before - we probably crashed or the chain timed out!");
                continue;
            }
            warn!(
                "we have already generated dealing {dealing_index} before, but failed to submit it"
            );
            let contract_dealing =
                PartialContractDealing::new(*dealing_index, ContractDealing::from(dealing));

            self.dkg_client
                .submit_dealing(contract_dealing, resharing)
                .await?;
        }
        Ok(())
    }

    /// Check whether this dealer can participate in the resharing
    /// by looking into the contract and ensuring it's in the list of initial dealers for this epoch
    async fn can_reshare(&self) -> Result<bool, CoconutError> {
        let Some(initial_data) = self.dkg_client.get_initial_dealers().await? else {
            return Ok(false);
        };

        let address = self.dkg_client.get_address().await;
        Ok(initial_data
            .initial_dealers
            .iter()
            .any(|d| d.as_str() == address.as_ref()))
    }

    async fn handle_resharing_with_prior_key(
        &mut self,
        epoch_id: EpochId,
        expected_key_size: u32,
        old_keypair: KeyPairWithEpoch,
    ) -> Result<(), CoconutError> {
        // make sure we're allowed to participate in resharing
        if !self.can_reshare().await? {
            // we have to wait for other dealers to give us the dealings (hopefully)
            warn!("we we have an existing coconut keypair, but we're not allowed to participate in resharing");
            return Ok(());
        }

        // EDGE CASE:
        // make sure our keypair is from strictly the previous epoch
        // because our node might have been offline for multiple epochs and while we do have a coconut keypair,
        // it could be outdated and we can't use it for resharing
        let previous = epoch_id.saturating_sub(1);
        if old_keypair.issued_for_epoch != previous {
            warn!("our existing coconut keypair has been generated for an distant epoch ({} vs expected {previous} for resharing)", old_keypair.issued_for_epoch);
            // don't participate in resharing
            return Ok(());
        }

        // EDGE CASE:
        // we have changed the key size (because we wanted to add new attribute to credentials)
        // in this instance we can't reuse our key and have to generate brand new dealings
        if expected_key_size != 1 + old_keypair.keys.secret_key().size() as u32 {
            warn!("our existing coconut keypair has different size than the currently expected value ({expected_key_size} vs {})", old_keypair.keys.secret_key().size() as u32);
            self.generate_fresh_dealings(epoch_id, expected_key_size)
                .await?;
            return Ok(());
        }

        // generate resharing dealings
        let prior_secrets = old_keypair.hazmat_into_secrets();
        // safety:
        // the prior secrets will be immediately converted into `Polynomial` with the specified coefficient
        // that does implement `ZeroizeOnDrop`
        self.generate_resharing_dealings(epoch_id, prior_secrets)
            .await?;

        Ok(())
    }

    pub(crate) async fn dealing_exchange(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), CoconutError> {
        let dealing_state = self.state.dealing_exchange_state(epoch_id)?;

        // check if we have already submitted the dealings
        if dealing_state.completed() {
            // the only way this could be a false positive is if the chain forked and blocks got reverted,
            // but I don't think we have to worry about that
            debug!("we have already submitted all the dealings for this epoch");
            return Ok(());
        }

        // FAILURE CASE:
        // check if we have already generated the dealings, but they failed to get sent to the contract for whatever reason
        if !dealing_state.generated_dealings.is_empty() {
            debug!("we have already generated the dealings for this epoch");
            self.resubmit_pregenerated_dealings(epoch_id, resharing)
                .await?;

            // if we managed to resubmit the dealings (i.e. we didn't return an error),
            // it means the state is complete now.
            self.state.dealing_exchange_state_mut(epoch_id)?.completed = true;
            return Ok(());
        }

        // we don't have any prior information - grab, parse and cache it since we will need it in next steps
        // and it's not going to change during the epoch
        let dealers = self.dkg_client.get_current_dealers().await?;

        // EDGE CASE:
        // if there are no dealers for some reason, don't attempt to generate dealings as this will fail with a panic
        if dealers.is_empty() {
            warn!("there are no active dealers/receivers to generate dealings for");
            self.state.dealing_exchange_state_mut(epoch_id)?.completed = true;
            return Ok(());
        }

        self.state.dkg_state_mut(epoch_id)?.set_dealers(dealers);

        // get the expected key size which will determine the number of dealings we need to construct
        let contract_state = self.dkg_client.get_contract_state().await?;
        let expected_key_size = contract_state.key_size;

        // there are few cases to cover here based on the resharing status and presence of coconut keypair:
        // - resharing + we have a key => we should use the prior secrets for the resharing dealings generation
        // - resharing + we don't have a key => we are a new party that joined the existing setup and we have to wait for others to give us the shares
        // - no resharing + we have a key => whole DKG has been restarted (probably enough new parties joined / old parties left) to trigger it
        // - no resharing + we don't have a key => either as above (but we're a new party) or it's the very first instance of the DKG
        if let Some(old_keypair) = self.state.take_coconut_keypair().await {
            let keypair_epoch = old_keypair.issued_for_epoch;

            if resharing {
                debug!("resharing + prior key");
                self.handle_resharing_with_prior_key(epoch_id, expected_key_size, old_keypair)
                    .await?;
            } else {
                debug!("no resharing + prior key");
                self.generate_fresh_dealings(epoch_id, expected_key_size)
                    .await?;
            }

            // EDGE CASE:
            // make sure to persist the state after possibly generating the resharing dealings as we're going to be archiving the keypair
            // (so we won't be able to create resharing dealings again if we crashed since we won't be able to load the keys)
            self.state.persist()?;
            // archive the keypair
            if let Err(source) = archive_coconut_keypair(&self.coconut_key_path, keypair_epoch) {
                return Err(CoconutError::KeyArchiveFailure {
                    epoch_id,
                    path: self.coconut_key_path.clone(),
                    source,
                });
            }
        } else {
            // sure, the if statements could be collapsed, but i prefer to explicitly repeat the block for readability
            if resharing {
                debug!("resharing + no prior key -> nothing to do");
                if self.can_reshare().await? {
                    warn!("this dealer was expected to participate in resharing but it doesn't have any prior keys to use");
                }
            } else {
                debug!("no resharing + no prior key");
                self.generate_fresh_dealings(epoch_id, expected_key_size)
                    .await?;
            }
        }

        // if we have generated any dealings => submit them, otherwise we're done
        let dealings = &self
            .state
            .dealing_exchange_state(epoch_id)?
            .generated_dealings;
        let total = dealings.len();
        for (i, (dealing_index, dealing)) in dealings.iter().enumerate() {
            let i = i + 1;
            debug!("submitting dealing index {dealing_index} ({i}/{total})");

            let contract_dealing =
                PartialContractDealing::new(*dealing_index, ContractDealing::from(dealing));

            self.dkg_client
                .submit_dealing(contract_dealing, resharing)
                .await?;
        }

        self.state.dealing_exchange_state_mut(epoch_id)?.completed = true;
        info!("DKG: Finished dealing exchange");
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::coconut::dkg::complaints::ComplaintReason;
    use crate::coconut::dkg::state::PersistentState;
    use crate::coconut::tests::fixtures::{
        dealers_fixtures, test_rng, TestingDkgControllerBuilder,
    };
    use crate::coconut::tests::helpers::unchecked_decode_bte_key;
    use crate::coconut::tests::DummyClient;
    use crate::coconut::KeyPair;
    use cosmwasm_std::Addr;
    use nym_coconut::{ttp_keygen, Parameters};
    use nym_coconut_dkg_common::dealer::DealerDetails;
    use nym_coconut_dkg_common::types::InitialReplacementData;
    use nym_crypto::asymmetric::identity;
    use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
    use nym_dkg::bte::{Params, PublicKeyWithProof};
    use nym_validator_client::nyxd::AccountId;
    use rand::rngs::OsRng;
    use rand_07::thread_rng;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::sync::{Arc, RwLock};
    use url::Url;
    //
    // const TEST_VALIDATORS_ADDRESS: [&str; 4] = [
    //     "n1aq9kakfgwqcufr23lsv644apavcntrsqsk4yus",
    //     "n1s9l3xr4g0rglvk4yctktmck3h4eq0gp6z2e20v",
    //     "n19kl4py32vsk297dm93ezem992cdyzdy4zuc2x6",
    //     "n1jfrs6cmw9t7dv0x8cgny6geunzjh56n2s89fkv",
    // ];
    //
    // fn insert_dealers(
    //     params: &Params,
    //     dealer_details_db: &Arc<RwLock<HashMap<String, (DealerDetails, bool)>>>,
    // ) -> Vec<DkgKeyPair> {
    //     let mut keypairs = vec![];
    //     for (idx, addr) in TEST_VALIDATORS_ADDRESS.iter().enumerate() {
    //         let keypair = DkgKeyPair::new(params, OsRng);
    //         let identity_keypair = identity::KeyPair::new(&mut thread_rng());
    //
    //         let bte_public_key_with_proof =
    //             bs58::encode(&keypair.public_key().to_bytes()).into_string();
    //         keypairs.push(keypair);
    //         dealer_details_db.write().unwrap().insert(
    //             addr.to_string(),
    //             (
    //                 DealerDetails {
    //                     address: Addr::unchecked(*addr),
    //                     bte_public_key_with_proof,
    //                     ed25519_identity: identity_keypair.public_key().to_base58_string(),
    //                     announce_address: format!("localhost:80{}", idx),
    //                     assigned_index: (idx + 1) as u64,
    //                 },
    //                 true,
    //             ),
    //         );
    //     }
    //     keypairs
    // }

    #[tokio::test]
    #[ignore] // expensive test
    async fn exchange_dealing() -> anyhow::Result<()> {
        let mut rng = test_rng([69u8; 32]);
        let dealers = dealers_fixtures(&mut rng, 4);
        let self_dealer = dealers[0].clone();

        let mut controller = TestingDkgControllerBuilder::default()
            .with_threshold(2)
            .with_dealers(dealers.clone())
            .with_as_dealer(self_dealer.clone())
            .build();

        let epoch = controller.dkg_client.get_current_epoch().await?.epoch_id;
        let key_size = controller.dkg_client.get_contract_state().await?.key_size;

        // initial state
        assert!(controller.state.dkg_state(epoch)?.dealers.is_empty());
        assert!(controller
            .state
            .dealing_exchange_state(epoch)?
            .generated_dealings
            .is_empty());

        // exchange
        controller.dealing_exchange(epoch, false).await.unwrap();

        let expected_dealers = dealers
            .iter()
            .map(|d| {
                (
                    d.assigned_index,
                    unchecked_decode_bte_key(&d.bte_public_key_with_proof),
                )
            })
            .collect::<Vec<_>>();
        let dealers = controller
            .state
            .dkg_state(epoch)?
            .dealers
            .values()
            .map(|p| (p.assigned_index, p.unwrap_key()))
            .collect::<Vec<_>>();

        let generated_dealings = controller
            .state
            .dealing_exchange_state(epoch)?
            .generated_dealings
            .clone();

        assert_eq!(expected_dealers, dealers);
        assert_eq!(key_size as usize, generated_dealings.len());

        // also make sure the fake chain state contains our dealings (since we submitted them)
        let chain_state = controller.chain_state.lock().unwrap();

        let submitted_dealings = chain_state
            .dealings
            .get(&epoch)
            .unwrap()
            .get(self_dealer.address.as_str())
            .unwrap();

        for submitted_dealing in submitted_dealings {
            let dealing = Dealing::try_from_bytes(submitted_dealing.data.as_slice())?;
            assert_eq!(
                generated_dealings.get(&submitted_dealing.index).unwrap(),
                &dealing
            )
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn invalid_bte_proof_dealing_posted() {
        todo!()
        // let self_index = 2;
        // let dealer_details_db = Arc::new(RwLock::new(HashMap::new()));
        // let dealings_db = Arc::new(RwLock::new(HashMap::new()));
        // let threshold_db = Arc::new(RwLock::new(Some(2)));
        // let dkg_client = DkgClient::new(
        //     DummyClient::new(AccountId::from_str(TEST_VALIDATORS_ADDRESS[0]).unwrap())
        //         .with_dealer_details(&dealer_details_db)
        //         .with_dealings(&dealings_db)
        //         .with_threshold(&threshold_db),
        // );
        // let params = dkg::params();
        // let identity_keypair = identity::KeyPair::new(&mut thread_rng());
        // let mut state = State::new(
        //     PathBuf::default(),
        //     PersistentState::default(),
        //     Url::parse("localhost:8000").unwrap(),
        //     DkgKeyPair::new(params, OsRng),
        //     *identity_keypair.public_key(),
        //     KeyPair::new(),
        // );
        // state.set_node_index(Some(self_index));
        // insert_dealers(params, &dealer_details_db);
        //
        // dealer_details_db
        //     .write()
        //     .unwrap()
        //     .entry(TEST_VALIDATORS_ADDRESS[1].to_string())
        //     .and_modify(|details| {
        //         let mut bytes = bs58::decode(details.0.bte_public_key_with_proof.clone())
        //             .into_vec()
        //             .unwrap();
        //         // Find another value for last byte that still deserializes to a public key with proof
        //         let initial_byte = *bytes.last_mut().unwrap();
        //         loop {
        //             let last_byte = bytes.last_mut().unwrap();
        //             let (ret, _) = last_byte.overflowing_add(1);
        //             *last_byte = ret;
        //             // stop when we find that value, or if we do a full round trip of u8 values
        //             // and can't find one, in which case this test is invalid
        //             if PublicKeyWithProof::try_from_bytes(&bytes).is_ok() || ret == initial_byte {
        //                 break;
        //             }
        //         }
        //         details.0.bte_public_key_with_proof = bs58::encode(&bytes).into_string();
        //     });
        //
        // let mut controller = DkgController::test_mock(dkg_client, state);
        // controller.dealing_exchange(0, false).await.unwrap();
        // assert_eq!(
        //     *controller
        //         .state
        //         .all_dealers()
        //         .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[1]))
        //         .unwrap()
        //         .as_ref()
        //         .unwrap_err(),
        //     ComplaintReason::InvalidBTEPublicKey
        // );
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn resharing_exchange_dealing() {
        todo!()
        // let self_index = 2;
        // let dealer_details_db = Arc::new(RwLock::new(HashMap::new()));
        // let dealings_db = Arc::new(RwLock::new(HashMap::new()));
        // let threshold_db = Arc::new(RwLock::new(Some(3)));
        // let initial_dealers_db = Arc::new(RwLock::new(Some(InitialReplacementData {
        //     initial_dealers: vec![Addr::unchecked(TEST_VALIDATORS_ADDRESS[0])],
        //     initial_height: 100,
        // })));
        // let dkg_client = DkgClient::new(
        //     DummyClient::new(
        //         AccountId::from_str("n1vxkywf9g4cg0k2dehanzwzz64jw782qm0kuynf").unwrap(),
        //     )
        //     .with_dealer_details(&dealer_details_db)
        //     .with_dealings(&dealings_db)
        //     .with_threshold(&threshold_db)
        //     .with_initial_dealers_db(&initial_dealers_db),
        // );
        // let contract_state = dkg_client.get_contract_state().await.unwrap();
        //
        // let params = dkg::params();
        // let mut keys = ttp_keygen(&Parameters::new(4).unwrap(), 3, 4).unwrap();
        // let coconut_keypair = KeyPair::new();
        // coconut_keypair.set(0, keys.pop().unwrap()).await;
        // let identity_keypair = identity::KeyPair::new(&mut thread_rng());
        //
        // let mut state = State::new(
        //     PathBuf::default(),
        //     PersistentState::default(),
        //     Url::parse("localhost:8000").unwrap(),
        //     DkgKeyPair::new(params, OsRng),
        //     *identity_keypair.public_key(),
        //     coconut_keypair.clone(),
        // );
        // state.set_node_index(Some(self_index));
        // let keypairs = insert_dealers(params, &dealer_details_db);
        // let mut controller = DkgController::test_mock(dkg_client, state);
        //
        // controller.dealing_exchange(0, true).await.unwrap();
        //
        // assert_eq!(
        //     controller
        //         .state
        //         .current_dealers_by_idx()
        //         .values()
        //         .collect::<Vec<_>>(),
        //     keypairs
        //         .iter()
        //         .map(|k| k.public_key().public_key())
        //         .collect::<Vec<_>>()
        // );
        // assert_eq!(state.threshold().unwrap(), 3);
        // assert_eq!(state.receiver_index().unwrap(), 1);
        // // let addr = dkg_client.get_address().await;
        //
        // // no dealings submitted for the first (zeroth) epoch
        // assert!(dealings_db.read().unwrap().get(&0).is_none());
        //
        // let identity_keypair = identity::KeyPair::new(&mut thread_rng());
        // let mut state = State::new(
        //     PathBuf::default(),
        //     PersistentState::default(),
        //     Url::parse("localhost:8000").unwrap(),
        //     DkgKeyPair::new(params, OsRng),
        //     *identity_keypair.public_key(),
        //     coconut_keypair,
        // );
        // state.set_node_index(Some(self_index));
        // // Use a client that is in the initial dealers set
        // let dkg_client = DkgClient::new(
        //     DummyClient::new(AccountId::from_str(TEST_VALIDATORS_ADDRESS[0]).unwrap())
        //         .with_dealer_details(&dealer_details_db)
        //         .with_dealings(&dealings_db)
        //         .with_threshold(&threshold_db)
        //         .with_initial_dealers_db(&initial_dealers_db),
        // );
        //
        // let mut controller = DkgController::test_mock(dkg_client, state);
        // controller.dealing_exchange(0, true).await.unwrap();
        //
        // let dealings = dealings_db
        //     .read()
        //     .unwrap()
        //     .get(&0)
        //     .unwrap()
        //     .get(TEST_VALIDATORS_ADDRESS[0])
        //     .unwrap()
        //     .clone();
        // assert_eq!(dealings.len(), contract_state.key_size as usize);
    }
}
