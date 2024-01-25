// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::dkg;
use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::controller::DkgController;
use crate::coconut::dkg::state::State;
use crate::coconut::keys::KeyPair;
use crate::coconut::tests::{DummyClient, SharedFakeChain};
use cosmwasm_std::Addr;
use nym_coconut_dkg_common::types::DealerDetails;
use nym_crypto::asymmetric::identity;
use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
use nym_dkg::{NodeIndex, Threshold};
use nym_validator_client::nyxd::AccountId;
use rand_chacha::{
    rand_core::{RngCore, SeedableRng},
    ChaCha20Rng,
};
use std::ops::{Deref, DerefMut};
use tempfile::{tempdir, TempDir};

pub fn test_rng(seed: [u8; 32]) -> ChaCha20Rng {
    ChaCha20Rng::from_seed(seed)
}

pub fn test_rng_07(seed: [u8; 32]) -> rand_chacha_02::ChaCha20Rng {
    use rand_chacha_02::rand_core::SeedableRng;
    rand_chacha_02::ChaCha20Rng::from_seed(seed)
}

pub fn pseudorandom_account(rng: &mut ChaCha20Rng) -> AccountId {
    let mut dummy_account_key_hash = [0u8; 32];
    rng.fill_bytes(&mut dummy_account_key_hash);
    AccountId::new("n", &dummy_account_key_hash).unwrap()
}

pub fn dealer_fixture(mut rng: &mut ChaCha20Rng, id: NodeIndex) -> DealerDetails {
    // we might possibly need that private key later on
    let keypair = DkgKeyPair::new(dkg::params(), &mut rng);

    // lol, instantiate rng with an rng due to incompatibility, but even though it looks dodgy AF,
    // it's 100% deterministic
    let mut secondary_seed = [0u8; 32];
    rng.fill_bytes(&mut secondary_seed);

    let addr = pseudorandom_account(rng);
    let identity_keypair = identity::KeyPair::new(&mut test_rng_07(secondary_seed));
    let bte_public_key_with_proof = bs58::encode(&keypair.public_key().to_bytes()).into_string();

    let port = 8080 + id;
    DealerDetails {
        address: Addr::unchecked(addr.to_string()),
        bte_public_key_with_proof,
        ed25519_identity: identity_keypair.public_key().to_base58_string(),
        announce_address: format!("http://localhost:{port}"),
        assigned_index: id,
    }
}

pub fn dealers_fixtures(rng: &mut ChaCha20Rng, n: usize) -> Vec<DealerDetails> {
    let mut dealers = Vec::new();
    for i in 1..=n {
        dealers.push(dealer_fixture(rng, i as NodeIndex))
    }
    dealers
}

#[derive(Default)]
pub struct TestingDkgControllerBuilder {
    rng: Option<ChaCha20Rng>,
    rng_seed: Option<[u8; 32]>,
    address: Option<AccountId>,
    threshold: Option<Threshold>,

    chain_state: Option<SharedFakeChain>,

    self_dealer: Option<DealerDetails>,
    dealers: Vec<DealerDetails>,
}

impl TestingDkgControllerBuilder {
    pub fn with_magic_seed_val(mut self, val: u8) -> Self {
        self.rng_seed = Some([val; 32]);
        self
    }

    #[allow(dead_code)]
    pub fn with_rng(mut self, rng: ChaCha20Rng) -> Self {
        self.rng = Some(rng);
        self
    }

    pub fn with_shared_chain_state(mut self, fake_chain: SharedFakeChain) -> Self {
        self.chain_state = Some(fake_chain);
        self
    }

    pub fn with_as_dealer(mut self, dealer_details: DealerDetails) -> Self {
        self.self_dealer = Some(dealer_details);
        self
    }

    #[allow(dead_code)]
    pub fn with_dealer(mut self, dealer_details: DealerDetails) -> Self {
        self.dealers.push(dealer_details);
        self
    }

    pub fn with_dealers(mut self, dealers_details: Vec<DealerDetails>) -> Self {
        self.dealers = dealers_details;
        self
    }

    #[allow(dead_code)]
    pub fn with_address(mut self, address: impl Into<String>) -> Self {
        let addr = address.into();
        self.address = Some(addr.parse().unwrap());
        self
    }

    pub fn with_threshold(mut self, threshold: Threshold) -> Self {
        self.threshold = Some(threshold);
        self
    }

    pub fn build(self) -> TestingDkgController {
        let mut rng = self.rng.unwrap_or_else(|| {
            let rng_seed = self.rng_seed.unwrap_or([69u8; 32]);
            test_rng(rng_seed)
        });

        let had_dealer_info = self.self_dealer.is_some();

        // is this ideal? no, but it works : P
        let self_dealer = self.self_dealer.unwrap_or_else(|| {
            let address = self
                .address
                .unwrap_or_else(|| pseudorandom_account(&mut rng));
            let mut secondary_seed = [0u8; 32];
            rng.fill_bytes(&mut secondary_seed);

            let identity_keypair = identity::KeyPair::new(&mut test_rng_07(secondary_seed));

            DealerDetails {
                address: Addr::unchecked(address.as_ref()),
                bte_public_key_with_proof: "foomp".to_string(),
                ed25519_identity: identity_keypair.public_key().to_base58_string(),
                announce_address: "http://localhost:8080".to_string(),
                assigned_index: 1,
            }
        });

        let chain_state = self.chain_state.unwrap_or_default();
        let dummy_client = DummyClient::new(
            self_dealer.address.to_string().parse().unwrap(),
            chain_state.clone(),
        );
        // insert initial data into the chain state

        let mut state_guard = chain_state.lock().unwrap();
        if let Some(threshold) = self.threshold {
            state_guard.dkg_contract.threshold = Some(threshold)
        }
        for dealer in self.dealers {
            state_guard
                .dkg_contract
                .dealers
                .insert(dealer.assigned_index, dealer);
        }

        let epoch = state_guard.dkg_contract.epoch.epoch_id;
        drop(state_guard);

        let dummy_client = DkgClient::new(dummy_client);
        let tmp_dir = tempdir().unwrap();

        let mut state = State::new(
            tmp_dir.path().join("persistent_state.json"),
            Default::default(),
            self_dealer.announce_address.parse().unwrap(),
            // TODO: we might need to fix up the key here
            DkgKeyPair::new(&nym_dkg::bte::setup(), &mut rng),
            self_dealer.ed25519_identity.parse().unwrap(),
            KeyPair::new(),
        );

        if had_dealer_info {
            // if we had dealer info it means we must have gone through key registration
            state.maybe_init_dkg_state(epoch);
            state.registration_state_mut(epoch).unwrap().assigned_index =
                Some(self_dealer.assigned_index);
        }

        TestingDkgController {
            controller: DkgController::test_mock(
                rng,
                dummy_client,
                state,
                tmp_dir.path().join("coconut_keypair.pem"),
            ),
            chain_state,
            _tmp_dir: tmp_dir,
        }
    }
}

pub fn dkg_controller_fixture() -> TestingDkgController {
    TestingDkgControllerBuilder::default().build()
}

pub(crate) struct TestingDkgController {
    pub(crate) controller: DkgController<ChaCha20Rng>,

    pub(crate) chain_state: SharedFakeChain,

    _tmp_dir: TempDir,
}

impl Deref for TestingDkgController {
    type Target = DkgController<ChaCha20Rng>;

    fn deref(&self) -> &Self::Target {
        &self.controller
    }
}

impl DerefMut for TestingDkgController {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.controller
    }
}
