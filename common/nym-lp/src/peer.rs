// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use libcrux_kem::{MlKem768PrivateKey, MlKem768PublicKey};
use libcrux_psq::handshake::Responder;
use libcrux_psq::handshake::types::{DHKeyPair, DHPrivateKey, DHPublicKey};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kkt::ciphersuite::{
    Ciphersuite, DecapsulationKey, EncapsulationKey, KEM, KEMKeyDigests, KemKeyPair,
    SignatureScheme, SigningKeyDigests,
};
use rand::rngs::ThreadRng;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use crate::psq::build_responder;

/// Representation of a local Lewes Protocol peer
/// encapsulating all the known information and keys.
#[derive(Clone)]
pub struct LpLocalPeer {
    pub(crate) ciphersuite: Ciphersuite,

    /// Local Ed25519 keys for PSQ authentication
    pub(crate) ed25519: Arc<ed25519::KeyPair>,

    /// Local x25519 keys (Noise static key)
    pub(crate) x25519: Arc<DHKeyPair>,

    /// Local KEM keys used for PSQ
    pub(crate) kem_keypairs: HashMap<KEM, Arc<KemKeyPair>>,
}

impl LpLocalPeer {
    pub fn new(
        ciphersuite: Ciphersuite,
        ed25519: Arc<ed25519::KeyPair>,
        x25519: Arc<x25519::KeyPair>,
    ) -> Self {
        // TODO: make nicer conversion (without cloning) + error handling
        let initiator_libcrux_x25519_private_key =
            DHPrivateKey::from_bytes(x25519.private_key().as_bytes()).unwrap();
        let initiator_x25519_keypair = DHKeyPair::from(initiator_libcrux_x25519_private_key);

        LpLocalPeer {
            ciphersuite,
            ed25519,
            x25519: Arc::new(initiator_x25519_keypair),
            kem_keypairs: Default::default(),
        }
    }

    pub fn build_client_hello_data(&self, timestamp: u64) -> ClientHelloData {
        ClientHelloData::new_with_fresh_salt(
            *self.x25519().public_key(),
            *self.ed25519().public_key(),
            timestamp,
        )
    }

    pub fn with_kem_keypair(mut self, keypair: Arc<KemKeyPair>) -> Self {
        let kem = keypair.kem();
        self.kem_keypairs.insert(kem, keypair);
        self
    }

    pub fn ed25519(&self) -> &Arc<ed25519::KeyPair> {
        &self.ed25519
    }

    pub fn x25519(&self) -> &Arc<DHKeyPair> {
        &self.x25519
    }

    // /// Returns the reference to the KEM Public key of the peer (if available).
    // pub fn get_kem_key_handle(&self) -> Result<&x25519::PublicKey, LpError> {
    //     self.kem_psq
    //         .as_ref()
    //         .map(|kp| kp.public_key())
    //         .ok_or(LpError::ResponderWithMissingKEMKey)
    // }

    pub fn kem_key(&self, kem: KEM) -> Option<&Arc<KemKeyPair>> {
        self.kem_keypairs.get(&kem)
    }

    /// Convert this `LpLocalPeer` into a valid `LpRemotePeer` that can be used within tests
    #[doc(hidden)]
    pub fn as_remote(&self) -> LpRemotePeer {
        let mut expected_signing_key_digests = HashMap::new();
        expected_signing_key_digests.insert(
            SignatureScheme::Ed25519,
            nym_kkt::key_utils::produce_key_digests(self.ed25519.public_key().as_bytes()),
        );

        let mut expected_kem_key_digests = HashMap::new();
        for (kem, kem_key) in &self.kem_keypairs {
            expected_kem_key_digests.insert(
                *kem,
                nym_kkt::key_utils::produce_key_digests(&kem_key.encoded_encapsulation_key()),
            );
        }

        LpRemotePeer {
            ed25519_public: *self.ed25519.public_key(),
            x25519_public: self.x25519.pk,
            expected_kem_key_digests,
            expected_signing_key_digests,
        }
    }
}

impl Debug for LpLocalPeer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LpLocalPeer")
            .field("ciphersuite", &self.ciphersuite)
            .field("ed25519", &self.ed25519)
            .field("x25519", &self.x25519.pk)
            .field("kem_keypairs", &self.kem_keypairs)
            .finish()
    }
}

/// Representation of a remote Lewes Protocol peer
/// encapsulating all the known information and keys.
#[derive(Debug, Clone)]
pub struct LpRemotePeer {
    /// Remote Ed25519 public key for PSQ authentication
    pub(crate) ed25519_public: ed25519::PublicKey,

    /// Remote X25519 public key (Noise static key)
    pub(crate) x25519_public: DHPublicKey,

    /// Expected digests of the remote's KEM key
    pub(crate) expected_kem_key_digests: HashMap<KEM, KEMKeyDigests>,

    /// Expected digests of the remote's signing key
    pub(crate) expected_signing_key_digests: HashMap<SignatureScheme, SigningKeyDigests>,
}

impl LpRemotePeer {
    pub fn new(ed25519_public: ed25519::PublicKey, x25519_public: x25519::PublicKey) -> Self {
        // TODO: make nicer conversion (without cloning) + error handling
        let responder_x25519_public_key = DHPublicKey::from_bytes(x25519_public.as_bytes());
        LpRemotePeer {
            ed25519_public,
            x25519_public: responder_x25519_public_key,
            expected_kem_key_digests: Default::default(),
            expected_signing_key_digests: Default::default(),
        }
    }

    pub fn ed25519(&self) -> ed25519::PublicKey {
        self.ed25519_public
    }

    pub fn x25519(&self) -> &DHPublicKey {
        &self.x25519_public
    }

    #[must_use]
    pub fn with_key_digests(
        mut self,
        expected_kem_key_digests: HashMap<KEM, KEMKeyDigests>,
        expected_signing_key_digests: HashMap<SignatureScheme, SigningKeyDigests>,
    ) -> Self {
        self.expected_kem_key_digests = expected_kem_key_digests;
        self.expected_signing_key_digests = expected_signing_key_digests;
        self
    }

    /// Attempt to retrieve expected KEM key hash of the remote
    /// for [`nym_kkt::ciphersuite::KEM`] key type and [`nym_kkt::ciphersuite::HashFunction`]
    /// specified by own [`nym_kkt::ciphersuite::Ciphersuite`]
    pub(crate) fn expected_kem_key_hash(
        &self,
        ciphersuite: Ciphersuite,
    ) -> Result<Vec<u8>, LpError> {
        let kem = ciphersuite.kem();
        let hash_function = ciphersuite.hash_function();

        let digests = self
            .expected_kem_key_digests
            .get(&kem)
            .ok_or(LpError::NoKnownKEMKeyDigests { kem, hash_function })?;

        digests
            .get(&hash_function)
            .ok_or(LpError::NoKnownKEMKeyDigests { kem, hash_function })
            .cloned()
    }
}

#[cfg(any(feature = "mock", test))]
pub fn mock_peer(kem: KEM) -> LpLocalPeer {
    // use deterministic rng
    let mut rng = nym_test_utils::helpers::deterministic_rng();

    let ciphersuite = Ciphersuite::new(
        kem,
        nym_kkt::ciphersuite::HashFunction::Blake3,
        SignatureScheme::Ed25519,
        nym_kkt::ciphersuite::HashLength::Default,
    );

    random_peer(&mut rng, ciphersuite)
}

#[cfg(any(feature = "mock", test))]
pub fn random_peer<'a, R: rand::CryptoRng + rand::RngCore>(
    rng: &mut R,
    ciphersuite: Ciphersuite,
) -> LpLocalPeer {
    use nym_kkt::key_utils::{generate_keypair_mceliece, generate_keypair_mlkem};
    let ed25519 = Arc::new(ed25519::KeyPair::new(rng));

    let mut sk = [0u8; 32];
    rng.fill_bytes(&mut sk);

    let TODO = "";
    // clamp
    sk[0] &= 248u8;
    sk[31] &= 127u8;
    sk[31] |= 64u8;

    let x25519 = Arc::new(DHKeyPair::from(DHPrivateKey::from_bytes(&sk).unwrap()));

    let default_peer = LpLocalPeer {
        ciphersuite: Arc::new(ciphersuite),
        ed25519,
        x25519,
        mlkem: None,
        mceliece: None,
    };

    match ciphersuite.kem() {
        KEM::MlKem768 => {
            let mlkem_keypair = generate_keypair_mlkem(&mut rand09::rng());
            default_peer.with_mlkem_keypair(&mlkem_keypair.0, &mlkem_keypair.1)
        }
        KEM::McEliece => {
            let mceliece_keypair = generate_keypair_mceliece(&mut rand09::rng());
            default_peer.with_mceliece_keypair(mceliece_keypair.0, mceliece_keypair.1)
        }
        _ => unreachable!(),
    }
}

#[cfg(any(feature = "mock", test))]
pub fn mock_peers(kem: KEM) -> (LpLocalPeer, LpLocalPeer) {
    println!("KEM: {:?}", kem);
    (mock_peer(kem), mock_peer(kem))
}
