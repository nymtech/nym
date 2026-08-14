// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Multi-controller DKG harness over the contract-backed chain.
//!
//! The contract-side counterpart of [`super::helpers`]: the same phase drivers, but
//! where those set epoch state and thresholds directly on the fake chain, these go
//! through the real contract - phases advance by passing the deadline and executing
//! `AdvanceEpochState`, the threshold is whatever the contract computed, and every
//! transition is asserted against the contract's own state machine.

use crate::ecash::dkg;
use crate::ecash::dkg::client::DkgClient;
use crate::ecash::dkg::controller::DkgController;
use crate::ecash::dkg::state::State;
use crate::ecash::keys::KeyPair;
use crate::ecash::tests::contract_chain::{ContractChainClient, SharedContractChain};
use crate::ecash::tests::fixtures::test_rng;
use cosmwasm_std::Addr;
use nym_coconut_dkg_common::msg::ExecuteMsg as DkgExecuteMsg;
use nym_coconut_dkg_common::types::EpochState;
use nym_compact_ecash::VerificationKeyAuth;
use nym_crypto::asymmetric::ed25519;
use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
use nym_validator_client::nyxd::AccountId;
use rand_chacha::ChaCha20Rng;
use std::ops::{Deref, DerefMut};
use tempfile::{tempdir, TempDir};

pub(crate) struct ContractDkgController {
    pub(crate) controller: DkgController<ChaCha20Rng>,
    pub(crate) chain: SharedContractChain,
    _tmp_dir: TempDir,
}

impl ContractDkgController {
    pub(crate) async fn address(&self) -> AccountId {
        self.dkg_client.get_address().await.unwrap()
    }

    pub(crate) async fn cw_address(&self) -> Addr {
        Addr::unchecked(self.address().await.as_ref())
    }

    pub(crate) async fn unchecked_coconut_vk(&self) -> VerificationKeyAuth {
        self.state
            .unchecked_coconut_keypair()
            .await
            .as_ref()
            .unwrap()
            .keys
            .verification_key()
            .clone()
    }
}

impl Deref for ContractDkgController {
    type Target = DkgController<ChaCha20Rng>;

    fn deref(&self) -> &Self::Target {
        &self.controller
    }
}

impl DerefMut for ContractDkgController {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.controller
    }
}

/// Build a controller whose chain-authenticated identity is `address` - it must be a
/// cw4 group member for registration to be accepted by the real contract.
pub(crate) fn initialise_controller(
    chain: &SharedContractChain,
    address: AccountId,
    seed: u8,
) -> ContractDkgController {
    let mut rng = test_rng([seed; 32]);
    let dkg_keypair = DkgKeyPair::new(dkg::params(), rng.clone());
    let identity_keypair = ed25519::KeyPair::new(&mut rng);
    let announce_address = format!("http://localhost:{}", 9000 + seed as u16);

    let tmp_dir = tempdir().unwrap();
    let state = State::new(
        tmp_dir.path().join("persistent_state.json"),
        Default::default(),
        announce_address.parse().unwrap(),
        dkg_keypair,
        *identity_keypair.public_key(),
        KeyPair::new(),
    );

    let client = DkgClient::new(ContractChainClient::new(address, chain.clone()));
    ContractDkgController {
        controller: DkgController::test_mock(
            rng,
            client,
            state,
            tmp_dir.path().join("coconut_keypair.pem"),
        ),
        chain: chain.clone(),
        _tmp_dir: tmp_dir,
    }
}

/// One controller per cw4 group member, in group order.
pub(crate) fn initialise_controllers(chain: &SharedContractChain) -> Vec<ContractDkgController> {
    chain
        .group_member_addresses()
        .into_iter()
        .enumerate()
        .map(|(i, address)| initialise_controller(chain, address, i as u8))
        .collect()
}

pub(crate) fn initiate_dkg(chain: &SharedContractChain) {
    chain
        .execute_dkg(chain.admin(), DkgExecuteMsg::InitiateDkg {})
        .unwrap();
    assert_eq!(
        chain.epoch().state,
        EpochState::PublicKeySubmission { resharing: false }
    );
}

pub(crate) fn trigger_resharing(chain: &SharedContractChain) {
    chain
        .execute_dkg(chain.admin(), DkgExecuteMsg::TriggerResharing {})
        .unwrap();
    assert_eq!(
        chain.epoch().state,
        EpochState::PublicKeySubmission { resharing: true }
    );
}

/// Move past the current phase's deadline and advance through the real transition
/// logic. The jump is longer than any phase duration but kept small enough that
/// pending multisig proposals (max voting period 3600 s) never expire mid-ceremony.
fn advance_state(chain: &SharedContractChain) {
    chain.advance_time_by(601);
    chain
        .execute_dkg(chain.admin(), DkgExecuteMsg::AdvanceEpochState {})
        .unwrap();
}

pub(crate) async fn submit_public_keys(controllers: &mut [ContractDkgController], resharing: bool) {
    let chain = controllers[0].chain.clone();
    let epoch_id = chain.epoch().epoch_id;

    for controller in controllers.iter_mut() {
        controller
            .public_key_submission(epoch_id, resharing)
            .await
            .unwrap();
    }

    advance_state(&chain);
    assert_eq!(
        chain.epoch().state,
        EpochState::DealingExchange { resharing }
    );
}

pub(crate) async fn exchange_dealings(controllers: &mut [ContractDkgController], resharing: bool) {
    let chain = controllers[0].chain.clone();
    let epoch_id = chain.epoch().epoch_id;

    for controller in controllers.iter_mut() {
        controller
            .dealing_exchange(epoch_id, resharing)
            .await
            .unwrap();
    }

    advance_state(&chain);
    assert_eq!(
        chain.epoch().state,
        EpochState::VerificationKeySubmission { resharing }
    );
}

pub(crate) async fn derive_keypairs(controllers: &mut [ContractDkgController], resharing: bool) {
    let chain = controllers[0].chain.clone();
    let epoch_id = chain.epoch().epoch_id;

    for controller in controllers.iter_mut() {
        controller
            .verification_key_submission(epoch_id, resharing)
            .await
            .unwrap();
    }

    advance_state(&chain);
    assert_eq!(
        chain.epoch().state,
        EpochState::VerificationKeyValidation { resharing }
    );
}

pub(crate) async fn validate_keys(controllers: &mut [ContractDkgController], resharing: bool) {
    let chain = controllers[0].chain.clone();
    let epoch_id = chain.epoch().epoch_id;

    for controller in controllers.iter_mut() {
        controller
            .verification_key_validation(epoch_id)
            .await
            .unwrap();
    }

    advance_state(&chain);
    assert_eq!(
        chain.epoch().state,
        EpochState::VerificationKeyFinalization { resharing }
    );
}

pub(crate) async fn finalize(controllers: &mut [ContractDkgController]) {
    let chain = controllers[0].chain.clone();
    let epoch_id = chain.epoch().epoch_id;

    for controller in controllers.iter_mut() {
        controller
            .verification_key_finalization(epoch_id)
            .await
            .unwrap();
    }

    advance_state(&chain);
    assert_eq!(chain.epoch().state, EpochState::InProgress);
}

/// Drive a complete ceremony through every phase of the real contract.
pub(crate) async fn run_full_ceremony(controllers: &mut [ContractDkgController], resharing: bool) {
    submit_public_keys(controllers, resharing).await;
    exchange_dealings(controllers, resharing).await;
    derive_keypairs(controllers, resharing).await;
    validate_keys(controllers, resharing).await;
    finalize(controllers).await;
}
