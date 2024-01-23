// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::dkg::controller::DkgController;
use crate::coconut::tests::fixtures::{TestingDkgController, TestingDkgControllerBuilder};
use crate::coconut::tests::FakeChainState;
use nym_coconut_dkg_common::types::EpochState;
use nym_crypto::aes::cipher::crypto_common::rand_core::OsRng;
use nym_dkg::bte::PublicKeyWithProof;
use nym_dkg::Dealing;
use std::env::temp_dir;
use std::sync::{Arc, Mutex};

pub(crate) fn unchecked_decode_bte_key(raw: &str) -> PublicKeyWithProof {
    let bytes = bs58::decode(raw).into_vec().unwrap();
    PublicKeyWithProof::try_from_bytes(&bytes).unwrap()
}

pub(crate) type SharedChainState = Arc<Mutex<FakeChainState>>;

pub(crate) fn init_chain() -> SharedChainState {
    Default::default()
}

pub(crate) fn initialise_controllers(amount: usize) -> Vec<TestingDkgController> {
    let chain = init_chain();

    let mut controllers = Vec::with_capacity(amount);
    assert!(amount <= u8::MAX as usize);
    for rng_seed in 0..amount {
        let controller = TestingDkgControllerBuilder::default()
            .with_shared_chain_state(chain.clone())
            .with_magic_seed_val(rng_seed as u8)
            .build();

        controllers.push(controller)
    }

    controllers
}

pub(crate) fn initialise_dkg(controllers: &mut [TestingDkgController], resharing: bool) {
    assert_eq!(
        controllers[0].chain_state.lock().unwrap().dkg_epoch.state,
        EpochState::WaitingInitialisation
    );

    controllers[0].chain_state.lock().unwrap().dkg_epoch.state =
        EpochState::PublicKeySubmission { resharing }
}

pub(crate) async fn submit_public_keys(controllers: &mut [TestingDkgController], resharing: bool) {
    let epoch = controllers[0]
        .chain_state
        .lock()
        .unwrap()
        .dkg_epoch
        .epoch_id;

    for controller in controllers.iter_mut() {
        controller
            .public_key_submission(epoch, resharing)
            .await
            .unwrap();
    }

    let threshold = (2 * controllers.len() as u64 + 3 - 1) / 3;

    let mut guard = controllers[0].chain_state.lock().unwrap();
    guard.dkg_epoch.state = EpochState::DealingExchange { resharing };
    guard.threshold = Some(threshold)
}

pub(crate) async fn exchange_dealings(controllers: &mut [TestingDkgController], resharing: bool) {
    let epoch = controllers[0]
        .chain_state
        .lock()
        .unwrap()
        .dkg_epoch
        .epoch_id;

    for controller in controllers.iter_mut() {
        controller.dealing_exchange(epoch, resharing).await.unwrap();
    }

    let mut guard = controllers[0].chain_state.lock().unwrap();
    guard.dkg_epoch.state = EpochState::VerificationKeySubmission { resharing };
}

pub(crate) async fn derive_keypairs(controllers: &mut [TestingDkgController], resharing: bool) {
    let epoch = controllers[0]
        .chain_state
        .lock()
        .unwrap()
        .dkg_epoch
        .epoch_id;

    for controller in controllers.iter_mut() {
        controller
            .verification_key_submission(epoch, resharing)
            .await
            .unwrap();
    }

    let mut guard = controllers[0].chain_state.lock().unwrap();
    guard.dkg_epoch.state = EpochState::VerificationKeyValidation { resharing }
}

pub(crate) async fn validate_keys(controllers: &mut [TestingDkgController], resharing: bool) {
    let epoch = controllers[0]
        .chain_state
        .lock()
        .unwrap()
        .dkg_epoch
        .epoch_id;

    for controller in controllers.iter_mut() {
        controller
            .verification_key_validation(epoch, resharing)
            .await
            .unwrap();
    }

    let mut guard = controllers[0].chain_state.lock().unwrap();
    guard.dkg_epoch.state = EpochState::VerificationKeyFinalization { resharing }
}

pub(crate) async fn finalize(controllers: &mut [TestingDkgController], resharing: bool) {
    let epoch = controllers[0]
        .chain_state
        .lock()
        .unwrap()
        .dkg_epoch
        .epoch_id;

    for controller in controllers.iter_mut() {
        todo!()
        // controller
        //     .verification_key_finalization(epoch, resharing)
        //     .await
        //     .unwrap();
    }

    let mut guard = controllers[0].chain_state.lock().unwrap();
    guard.dkg_epoch.state = EpochState::InProgress {}
}
