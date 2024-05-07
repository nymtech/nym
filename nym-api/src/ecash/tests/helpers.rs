// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::tests::fixtures::{TestingDkgController, TestingDkgControllerBuilder};
use crate::ecash::tests::SharedFakeChain;
use nym_coconut_dkg_common::types::EpochState;
use nym_dkg::bte::PublicKeyWithProof;

pub(crate) fn unchecked_decode_bte_key(raw: &str) -> PublicKeyWithProof {
    let bytes = bs58::decode(raw).into_vec().unwrap();
    PublicKeyWithProof::try_from_bytes(&bytes).unwrap()
}

pub(crate) fn init_chain() -> SharedFakeChain {
    Default::default()
}

pub(crate) async fn initialise_controllers(amount: usize) -> Vec<TestingDkgController> {
    let chain = init_chain();

    let mut controllers = Vec::with_capacity(amount);
    assert!(amount <= u8::MAX as usize);
    for rng_seed in 0..amount {
        let controller = initialise_controller(chain.clone(), rng_seed as u8).await;

        controllers.push(controller)
    }

    controllers
}

pub(crate) async fn initialise_controller(chain: SharedFakeChain, id: u8) -> TestingDkgController {
    TestingDkgControllerBuilder::default()
        .with_shared_chain_state(chain)
        .with_magic_seed_val(id)
        .build()
        .await
}

pub(crate) async fn initialise_dkg(controllers: &mut [TestingDkgController], resharing: bool) {
    assert_eq!(
        controllers[0]
            .chain_state
            .lock()
            .unwrap()
            .dkg_contract
            .epoch
            .state,
        EpochState::WaitingInitialisation
    );

    // add every dealer to group contract
    for controller in controllers.iter() {
        let address = controller.dkg_client.get_address().await;
        let mut chain = controllers[0].chain_state.lock().unwrap();
        chain.add_member(address.as_ref(), 10);
    }

    let mut chain = controllers[0].chain_state.lock().unwrap();
    chain.dkg_contract.epoch.state = EpochState::PublicKeySubmission { resharing }
}

pub(crate) async fn submit_public_keys(controllers: &mut [TestingDkgController], resharing: bool) {
    let epoch = controllers[0]
        .chain_state
        .lock()
        .unwrap()
        .dkg_contract
        .epoch
        .epoch_id;

    for controller in controllers.iter_mut() {
        controller
            .public_key_submission(epoch, resharing)
            .await
            .unwrap();
    }

    let threshold = (2 * controllers.len() as u64 + 3 - 1) / 3;

    let mut guard = controllers[0].chain_state.lock().unwrap();
    guard.dkg_contract.epoch.state = EpochState::DealingExchange { resharing };
    guard.dkg_contract.threshold = Some(threshold)
}

pub(crate) async fn exchange_dealings(controllers: &mut [TestingDkgController], resharing: bool) {
    let epoch = controllers[0]
        .chain_state
        .lock()
        .unwrap()
        .dkg_contract
        .epoch
        .epoch_id;

    for controller in controllers.iter_mut() {
        controller.dealing_exchange(epoch, resharing).await.unwrap();
    }

    let mut guard = controllers[0].chain_state.lock().unwrap();
    guard.dkg_contract.epoch.state = EpochState::VerificationKeySubmission { resharing };
}

pub(crate) async fn derive_keypairs(controllers: &mut [TestingDkgController], resharing: bool) {
    let epoch = controllers[0]
        .chain_state
        .lock()
        .unwrap()
        .dkg_contract
        .epoch
        .epoch_id;

    for controller in controllers.iter_mut() {
        controller
            .verification_key_submission(epoch, resharing)
            .await
            .unwrap();
    }

    let mut guard = controllers[0].chain_state.lock().unwrap();
    guard.dkg_contract.epoch.state = EpochState::VerificationKeyValidation { resharing }
}

pub(crate) async fn validate_keys(controllers: &mut [TestingDkgController], resharing: bool) {
    let epoch = controllers[0]
        .chain_state
        .lock()
        .unwrap()
        .dkg_contract
        .epoch
        .epoch_id;

    for controller in controllers.iter_mut() {
        controller.verification_key_validation(epoch).await.unwrap();
    }

    let mut guard = controllers[0].chain_state.lock().unwrap();
    guard.dkg_contract.epoch.state = EpochState::VerificationKeyFinalization { resharing }
}

pub(crate) async fn finalize(controllers: &mut [TestingDkgController]) {
    let epoch = controllers[0]
        .chain_state
        .lock()
        .unwrap()
        .dkg_contract
        .epoch
        .epoch_id;

    for controller in controllers.iter_mut() {
        controller
            .verification_key_finalization(epoch)
            .await
            .unwrap();
    }

    let mut guard = controllers[0].chain_state.lock().unwrap();
    guard.dkg_contract.epoch.state = EpochState::InProgress {}
}
