// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Multi-controller DKG harness over the contract-backed chain.
//!
//! The contract-side counterpart of [`super::helpers`]: the same phase drivers, but
//! where those set epoch state and thresholds directly on the fake chain, these go
//! through the real contract - phases advance by passing the deadline and executing
//! `AdvanceEpochState`, the threshold is whatever the contract computed, and every
//! transition is asserted against the contract's own state machine.

use crate::ecash::comm::QueryCommunicationChannel;
use crate::ecash::dkg;
use crate::ecash::dkg::client::DkgClient;
use crate::ecash::dkg::controller::DkgController;
use crate::ecash::dkg::state::State;
use crate::ecash::keys::KeyPair;
use crate::ecash::state::EcashState;
use crate::ecash::tests::contract_chain::{ContractChainClient, SharedContractChain};
use crate::ecash::tests::fixtures::test_rng;
use crate::support::storage::NymApiStorage;
use cosmwasm_std::Addr;
use nym_coconut_dkg_common::msg::ExecuteMsg as DkgExecuteMsg;
use nym_coconut_dkg_common::types::EpochState;
use nym_compact_ecash::VerificationKeyAuth;
use nym_crypto::asymmetric::ed25519;
use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
use nym_task::ShutdownManager;
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
pub(crate) fn advance_state(chain: &SharedContractChain) {
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

/// Finalize for every controller except `skipped`.
///
/// Each dealer executes its own verification proposal, so the skipped dealer's share is
/// left unverified on chain even though the epoch concludes normally - the state an
/// epoch ends up in when a signer drops out during the (60 second) finalization window.
pub(crate) async fn finalize_except(controllers: &mut [ContractDkgController], skipped: usize) {
    let chain = controllers[0].chain.clone();
    let epoch_id = chain.epoch().epoch_id;

    for (i, controller) in controllers.iter_mut().enumerate() {
        if i == skipped {
            continue;
        }
        controller
            .verification_key_finalization(epoch_id)
            .await
            .unwrap();
    }

    advance_state(&chain);
    assert_eq!(chain.epoch().state, EpochState::InProgress);
}

/// An [`EcashState`] whose chain and communication channel are both backed by the real
/// contract, with `signer_address` as this api's own cosmos identity.
///
/// Unlike [`super::build_dummy_ecash_state`], nothing here is stubbed above the chain:
/// the state resolves signers, thresholds and keys the way a deployed api would, so its
/// caches behave exactly as they do in production.
pub(crate) async fn contract_backed_ecash_state(
    chain: &SharedContractChain,
    signer_address: AccountId,
) -> EcashState {
    let mut rng = test_rng([1u8; 32]);
    let identity = ed25519::KeyPair::new(&mut rng);

    let mut config = crate::support::config::Config::new("test");
    config.ecash_signer.enabled = true;

    EcashState::new(
        &config,
        // the ecash contract plays no part in these tests
        chain.admin(),
        ContractChainClient::new(signer_address.clone(), chain.clone()),
        identity,
        KeyPair::new(),
        QueryCommunicationChannel::new(ContractChainClient::new(signer_address, chain.clone())),
        NymApiStorage::init_in_memory().await.unwrap(),
        &ShutdownManager::empty_mock(),
    )
}

/// A ceremony driven straight against the contract, with no DKG cryptography.
///
/// Every transition, guard and deadline here is still the contract's own - only the
/// payloads are placeholders, and no `DkgController` is involved. That makes it orders
/// of magnitude cheaper than [`run_full_ceremony`], which spends nearly all its time in
/// BTE key generation, dealing encryption and pairwise dealing verification.
///
/// Use this whenever a concluded (or in-flight) epoch is a *precondition* of the test.
/// Use the real ceremony when the cryptography is the subject: dealing validation, share
/// verification, or master key preservation across a resharing.
///
/// The placeholder shares do not parse as verification keys, so any test that reads them
/// back should finish with [`cheap::install_real_verification_keys`], which overwrites
/// them with a consistent set from `ttp_keygen` - trusted-dealer key generation being far
/// cheaper than running a mutually distrustful protocol for the same result.
pub(crate) mod cheap {
    use super::advance_state;
    use crate::ecash::tests::contract_chain::SharedContractChain;
    use nym_coconut_dkg_common::dealing::{DealingChunkInfo, PartialContractDealing};
    use nym_coconut_dkg_common::msg::ExecuteMsg as DkgExecuteMsg;
    use nym_coconut_dkg_common::types::EpochState;
    use nym_compact_ecash::{aggregate_verification_keys, ttp_keygen, Base58, VerificationKeyAuth};
    use nym_contracts_common::dealings::ContractSafeBytes;

    pub(crate) fn register_dealers(chain: &SharedContractChain, resharing: bool) {
        for member in chain.group_member_addresses() {
            chain
                .execute_dkg(
                    member.clone(),
                    DkgExecuteMsg::RegisterDealer {
                        bte_key_with_proof: format!("bte-key-{member}"),
                        identity_key: format!("identity-{member}"),
                        announce_address: format!("http://localhost:8080/{member}"),
                        resharing,
                    },
                )
                .unwrap();
        }
    }

    pub(crate) fn submit_dealings(chain: &SharedContractChain, resharing: bool) {
        for member in chain.group_member_addresses() {
            chain
                .execute_dkg(
                    member.clone(),
                    DkgExecuteMsg::CommitDealingsMetadata {
                        dealing_index: 1,
                        chunks: vec![DealingChunkInfo { size: 1 }],
                        resharing,
                    },
                )
                .unwrap();
            chain
                .execute_dkg(
                    member,
                    DkgExecuteMsg::CommitDealingsChunk {
                        chunk: PartialContractDealing {
                            dealing_index: 1,
                            chunk_index: 0,
                            data: ContractSafeBytes(vec![0]),
                        },
                    },
                )
                .unwrap();
        }
    }

    pub(crate) fn submit_vk_shares(chain: &SharedContractChain, resharing: bool) {
        for member in chain.group_member_addresses() {
            chain
                .execute_dkg(
                    member.clone(),
                    DkgExecuteMsg::CommitVerificationKeyShare {
                        share: format!("placeholder-vk-{member}"),
                        resharing,
                    },
                )
                .unwrap();
        }
    }

    /// Mark every share verified, as the multisig would once its proposals pass.
    pub(crate) fn verify_vk_shares(chain: &SharedContractChain, resharing: bool) {
        let multisig = chain.multisig_address();
        for member in chain.group_member_addresses() {
            chain
                .execute_dkg(
                    multisig.clone(),
                    DkgExecuteMsg::VerifyVerificationKeyShare {
                        owner: member.to_string(),
                        resharing,
                    },
                )
                .unwrap();
        }
    }

    pub(crate) fn advance(chain: &SharedContractChain) {
        advance_state(chain)
    }

    /// Replace the placeholder shares with a consistent set of real verification keys,
    /// returning the master key they aggregate to.
    pub(crate) fn install_real_verification_keys(
        chain: &SharedContractChain,
    ) -> VerificationKeyAuth {
        let epoch_id = chain.epoch().epoch_id;
        let members = chain.group_member_addresses();
        let threshold = chain
            .epoch_threshold(epoch_id)
            .expect("the contract set no threshold for this epoch");

        let keys = ttp_keygen(threshold, members.len() as u64).unwrap();

        let mut verification_keys = Vec::with_capacity(members.len());
        let mut indices = Vec::with_capacity(members.len());
        for member in &members {
            // the contract assigns dealer indices, and they are the x-coordinates the
            // shares must be aggregated against, so match them rather than assume order
            let node_index = chain.vk_share(epoch_id, member).node_index;
            let key = keys[(node_index - 1) as usize].verification_key().clone();

            chain.set_vk_share_value(epoch_id, member, key.to_bs58());
            verification_keys.push(key);
            indices.push(node_index);
        }

        aggregate_verification_keys(&verification_keys, Some(&indices)).unwrap()
    }

    /// Drive a whole ceremony from `PublicKeySubmission` to a concluded epoch.
    ///
    /// Tests that need to observe the epoch mid-flight should call the individual phases
    /// instead, so their observation points stay visible in the test itself.
    pub(crate) fn run_ceremony(chain: &SharedContractChain, resharing: bool) {
        register_dealers(chain, resharing);

        advance(chain);
        submit_dealings(chain, resharing);

        advance(chain);
        submit_vk_shares(chain, resharing);

        // VerificationKeySubmission => VerificationKeyValidation => VerificationKeyFinalization
        advance(chain);
        assert_eq!(
            chain.epoch().state,
            EpochState::VerificationKeyValidation { resharing }
        );
        advance(chain);

        verify_vk_shares(chain, resharing);
        advance(chain);
        assert_eq!(chain.epoch().state, EpochState::InProgress);
    }
}

/// Drive a complete ceremony through every phase of the real contract.
pub(crate) async fn run_full_ceremony(controllers: &mut [ContractDkgController], resharing: bool) {
    submit_public_keys(controllers, resharing).await;
    exchange_dealings(controllers, resharing).await;
    derive_keypairs(controllers, resharing).await;
    validate_keys(controllers, resharing).await;
    finalize(controllers).await;
}
