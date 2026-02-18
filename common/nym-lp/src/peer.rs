// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpError;
use libcrux_psq::handshake::types::{DHKeyPair, DHPublicKey};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kkt::keys::KEMKeys;
use nym_kkt_ciphersuite::{Ciphersuite, KEM, KEMKeyDigests, SignatureScheme, SigningKeyDigests};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

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
    pub(crate) kem_keypairs: Option<Arc<KEMKeys>>,
}

impl LpLocalPeer {
    pub fn new(
        ciphersuite: Ciphersuite,
        ed25519: Arc<ed25519::KeyPair>,
        x25519: Arc<DHKeyPair>,
    ) -> Self {
        LpLocalPeer {
            ciphersuite,
            ed25519,
            x25519,
            kem_keypairs: Default::default(),
        }
    }

    #[must_use]
    pub fn with_kem_keys(mut self, kem_keys: Arc<KEMKeys>) -> Self {
        self.kem_keypairs = Some(kem_keys);
        self
    }

    pub fn ed25519(&self) -> &Arc<ed25519::KeyPair> {
        &self.ed25519
    }

    pub fn x25519(&self) -> &Arc<DHKeyPair> {
        &self.x25519
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
        if let Some(keys) = &self.kem_keypairs {
            for kem in [KEM::MlKem768, KEM::McEliece] {
                expected_kem_key_digests.insert(
                    kem,
                    nym_kkt::key_utils::produce_key_digests(
                        keys.encoded_encapsulation_key(kem).unwrap(),
                    ),
                );
            }
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
pub fn mock_peer() -> LpLocalPeer {
    // use deterministic rng
    let mut rng = nym_test_utils::helpers::deterministic_rng();
    random_peer(&mut rng)
}

#[cfg(any(feature = "mock", test))]
pub fn random_peer<'a, R: rand::CryptoRng + rand::RngCore>(rng: &mut R) -> LpLocalPeer {
    let ed25519 = Arc::new(ed25519::KeyPair::new(rng));

    // disgusting conversion between rng08 and rng09
    let mut seed = [0u8; 32];
    rng.fill_bytes(&mut seed);

    let mut rng09 = nym_test_utils::helpers::seeded_rng_09(seed);

    let x25519 = Arc::new(nym_kkt::key_utils::generate_keypair_x25519(&mut rng09));

    LpLocalPeer {
        ciphersuite: Ciphersuite::default(),
        ed25519,
        x25519,
        kem_keypairs: Some(Arc::new(KEMKeys::new(
            nym_kkt::key_utils::generate_keypair_mceliece(&mut rng09),
            nym_kkt::key_utils::generate_keypair_mlkem(&mut rng09),
        ))),
    }
}

#[cfg(any(feature = "mock", test))]
pub fn mock_peers() -> (LpLocalPeer, LpLocalPeer) {
    let mut rng = nym_test_utils::helpers::deterministic_rng();

    (random_peer(&mut rng), random_peer(&mut rng))
}
