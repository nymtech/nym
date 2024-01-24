// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::OnceLock;

pub(crate) fn params() -> &'static nym_dkg::bte::Params {
    static PARAMS: OnceLock<nym_dkg::bte::Params> = OnceLock::new();
    PARAMS.get_or_init(nym_dkg::bte::setup)
}

pub(crate) mod client;
pub(crate) mod complaints;
pub(crate) mod controller;
pub(crate) mod dealing;
pub(crate) mod key_derivation;
pub(crate) mod key_finalization;
pub(crate) mod key_validation;
pub(crate) mod public_key;
pub(crate) mod state;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // expensive test
    async fn reshare_preserves_keys() {
        todo!()
        // let db = MockContractDb::new();
        // let mut clients_and_states = prepare_clients_and_states_with_finalization(&db).await;
        // for controller in clients_and_states.iter_mut() {
        //     controller.state.set_was_in_progress();
        // }
        //
        // let mut vks = vec![];
        // let mut indices = vec![];
        // for controller in clients_and_states.iter() {
        //     let vk = controller
        //         .state
        //         .coconut_keypair()
        //         .await
        //         .as_ref()
        //         .unwrap()
        //         .keys
        //         .verification_key()
        //         .clone();
        //     let index = controller.state.node_index().unwrap();
        //     vks.push(vk);
        //     indices.push(index);
        // }
        // let initial_master_vk = aggregate_verification_keys(&vks, Some(&indices)).unwrap();
        //
        // let new_dkg_client = DkgClient::new(
        //     DummyClient::new(
        //         AccountId::from_str("n1sqkxzh7nl6kgndr4ew9795t2nkwmd8tpql67q7").unwrap(),
        //     )
        //     .with_dealer_details(&db.dealer_details_db)
        //     .with_dealings(&db.dealings_db)
        //     .with_proposal_db(&db.proposal_db)
        //     .with_verification_share(&db.verification_share_db)
        //     .with_threshold(&db.threshold_db)
        //     .with_initial_dealers_db(&db.initial_dealers_db),
        // );
        // let keypair = DkgKeyPair::new(dkg::params(), OsRng);
        // let identity_keypair = identity::KeyPair::new(&mut thread_rng());
        // let state = State::new(
        //     PathBuf::default(),
        //     PersistentState::default(),
        //     Url::parse("localhost:8000").unwrap(),
        //     keypair,
        //     *identity_keypair.public_key(),
        //     KeyPair::new(),
        // );
        //
        // for (_, active) in db.dealer_details_db.write().unwrap().values_mut() {
        //     *active = false;
        // }
        //
        // *db.dealings_db.write().unwrap() = Default::default();
        // *db.verification_share_db.write().unwrap() = Default::default();
        // let mut initial_dealers = vec![];
        // for controller in clients_and_states.iter() {
        //     let client_address =
        //         Addr::unchecked(controller.dkg_client.get_address().await.as_ref());
        //     initial_dealers.push(client_address);
        // }
        // *db.initial_dealers_db.write().unwrap() = Some(InitialReplacementData {
        //     initial_dealers,
        //     initial_height: 1,
        // });
        // *clients_and_states.first_mut().unwrap() = DkgController::test_mock(new_dkg_client, state);
        //
        // for controller in clients_and_states.iter_mut() {
        //     controller.public_key_submission(0, true).await.unwrap();
        //     controller.dealing_exchange(0, true).await.unwrap();
        // }
        //
        // for controller in clients_and_states.iter_mut() {
        //     let random_file: usize = OsRng.gen();
        //     let keypath = temp_dir().join(format!("coconut{}.pem", random_file));
        //     verification_key_submission(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         &keypath,
        //         true,
        //     )
        //     .await
        //     .unwrap();
        //     std::fs::remove_file(keypath).unwrap();
        // }
        // for controller in clients_and_states.iter_mut() {
        //     verification_key_validation(&controller.dkg_client, &mut controller.state, true)
        //         .await
        //         .unwrap();
        // }
        // for controller in clients_and_states.iter_mut() {
        //     verification_key_finalization(&controller.dkg_client, &mut controller.state, true)
        //         .await
        //         .unwrap();
        // }
        // assert!(db
        //     .proposal_db
        //     .read()
        //     .unwrap()
        //     .values()
        //     .all(|proposal| { proposal.status == Status::Executed }));
        //
        // let mut vks = vec![];
        // let mut indices = vec![];
        // for controller in clients_and_states.iter() {
        //     let vk = controller
        //         .state
        //         .coconut_keypair()
        //         .await
        //         .as_ref()
        //         .unwrap()
        //         .keys
        //         .verification_key()
        //         .clone();
        //     let index = controller.state.node_index().unwrap();
        //     vks.push(vk);
        //     indices.push(index);
        // }
        // let reshared_master_vk = aggregate_verification_keys(&vks, Some(&indices)).unwrap();
        // assert_eq!(initial_master_vk, reshared_master_vk);
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn reshare_after_reset() {
        todo!()
        // let db = MockContractDb::new();
        // let mut clients_and_states = prepare_clients_and_states_with_finalization(&db).await;
        // for controller in clients_and_states.iter_mut() {
        //     controller.state.set_was_in_progress();
        // }
        //
        // let new_dkg_client = DkgClient::new(
        //     DummyClient::new(
        //         AccountId::from_str("n1vxkywf9g4cg0k2dehanzwzz64jw782qm0kuynf").unwrap(),
        //     )
        //     .with_dealer_details(&db.dealer_details_db)
        //     .with_dealings(&db.dealings_db)
        //     .with_proposal_db(&db.proposal_db)
        //     .with_verification_share(&db.verification_share_db)
        //     .with_threshold(&db.threshold_db)
        //     .with_initial_dealers_db(&db.initial_dealers_db),
        // );
        // let keypair = DkgKeyPair::new(dkg::params(), OsRng);
        // let identity_keypair = identity::KeyPair::new(&mut thread_rng());
        // let state = State::new(
        //     PathBuf::default(),
        //     PersistentState::default(),
        //     Url::parse("localhost:8000").unwrap(),
        //     keypair,
        //     *identity_keypair.public_key(),
        //     KeyPair::new(),
        // );
        // let new_dkg_client2 = DkgClient::new(
        //     DummyClient::new(
        //         AccountId::from_str("n1sqkxzh7nl6kgndr4ew9795t2nkwmd8tpql67q7").unwrap(),
        //     )
        //     .with_dealer_details(&db.dealer_details_db)
        //     .with_dealings(&db.dealings_db)
        //     .with_proposal_db(&db.proposal_db)
        //     .with_verification_share(&db.verification_share_db)
        //     .with_threshold(&db.threshold_db)
        //     .with_initial_dealers_db(&db.initial_dealers_db),
        // );
        // let keypair = DkgKeyPair::new(dkg::params(), OsRng);
        // let identity_keypair = identity::KeyPair::new(&mut thread_rng());
        // let state2 = State::new(
        //     PathBuf::default(),
        //     PersistentState::default(),
        //     Url::parse("localhost:8000").unwrap(),
        //     keypair,
        //     *identity_keypair.public_key(),
        //     KeyPair::new(),
        // );
        //
        // for (_, active) in db.dealer_details_db.write().unwrap().values_mut() {
        //     *active = false;
        // }
        //
        // *db.dealings_db.write().unwrap() = Default::default();
        // *db.verification_share_db.write().unwrap() = Default::default();
        // clients_and_states.pop().unwrap();
        // let controller2 = clients_and_states.pop().unwrap();
        // clients_and_states.push(DkgController::test_mock(new_dkg_client, state));
        // clients_and_states.push(DkgController::test_mock(new_dkg_client2, state2));
        //
        // // DKG in reset mode
        // for controller in clients_and_states.iter_mut() {
        //     controller.public_key_submission(0, false).await.unwrap();
        // }
        // for controller in clients_and_states.iter_mut() {
        //     controller.dealing_exchange(0, false).await.unwrap();
        // }
        // for controller in clients_and_states.iter_mut() {
        //     let random_file: usize = OsRng.gen();
        //     let keypath = temp_dir().join(format!("coconut{}.pem", random_file));
        //     verification_key_submission(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         &keypath,
        //         false,
        //     )
        //     .await
        //     .unwrap();
        //     std::fs::remove_file(keypath).unwrap();
        // }
        // for controller in clients_and_states.iter_mut() {
        //     verification_key_validation(&controller.dkg_client, &mut controller.state, false)
        //         .await
        //         .unwrap();
        // }
        // for controller in clients_and_states.iter_mut() {
        //     verification_key_finalization(&controller.dkg_client, &mut controller.state, false)
        //         .await
        //         .unwrap();
        // }
        // assert!(db
        //     .proposal_db
        //     .read()
        //     .unwrap()
        //     .values()
        //     .all(|proposal| { proposal.status == Status::Executed }));
        // for controller in clients_and_states.iter_mut() {
        //     controller.state.set_was_in_progress();
        // }
        //
        // // DKG in reshare mode
        // let mut vks = vec![];
        // let mut indices = vec![];
        // for controller in clients_and_states.iter() {
        //     let vk = controller
        //         .state
        //         .coconut_keypair()
        //         .await
        //         .as_ref()
        //         .unwrap()
        //         .keys
        //         .verification_key()
        //         .clone();
        //     let index = controller.state.node_index().unwrap();
        //     vks.push(vk);
        //     indices.push(index);
        // }
        // let initial_master_vk = aggregate_verification_keys(&vks, Some(&indices)).unwrap();
        //
        // for (_, active) in db.dealer_details_db.write().unwrap().values_mut() {
        //     *active = false;
        // }
        // *db.dealings_db.write().unwrap() = Default::default();
        // *db.verification_share_db.write().unwrap() = Default::default();
        // let mut initial_dealers = vec![];
        // for controller in clients_and_states.iter() {
        //     let client_address =
        //         Addr::unchecked(controller.dkg_client.get_address().await.as_ref());
        //     initial_dealers.push(client_address);
        // }
        // *db.initial_dealers_db.write().unwrap() = Some(InitialReplacementData {
        //     initial_dealers,
        //     initial_height: 1,
        // });
        // *clients_and_states.last_mut().unwrap() = controller2;
        //
        // for controller in clients_and_states.iter_mut() {
        //     controller.public_key_submission(0, true).await.unwrap();
        //     controller.dealing_exchange(0, true).await.unwrap();
        // }
        //
        // for controller in clients_and_states.iter_mut() {
        //     let random_file: usize = OsRng.gen();
        //     let keypath = temp_dir().join(format!("coconut{}.pem", random_file));
        //     verification_key_submission(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         &keypath,
        //         true,
        //     )
        //     .await
        //     .unwrap();
        //     std::fs::remove_file(keypath).unwrap();
        // }
        //
        // for controller in clients_and_states.iter_mut() {
        //     verification_key_validation(&controller.dkg_client, &mut controller.state, true)
        //         .await
        //         .unwrap();
        // }
        // for controller in clients_and_states.iter_mut() {
        //     verification_key_finalization(&controller.dkg_client, &mut controller.state, true)
        //         .await
        //         .unwrap();
        // }
        // // assert!(db
        // //     .proposal_db
        // //     .read()
        // //     .unwrap()
        // //     .values()
        // //     .all(|proposal| { proposal.status == Status::Executed }));
        //
        // let mut vks = vec![];
        // let mut indices = vec![];
        // for controller in clients_and_states.iter() {
        //     let vk = controller
        //         .state
        //         .coconut_keypair()
        //         .await
        //         .as_ref()
        //         .unwrap()
        //         .keys
        //         .verification_key()
        //         .clone();
        //     let index = controller.state.node_index().unwrap();
        //     vks.push(vk);
        //     indices.push(index);
        // }
        // let reshared_master_vk = aggregate_verification_keys(&vks, Some(&indices)).unwrap();
        // assert_eq!(initial_master_vk, reshared_master_vk);
    }
}
