// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpError;
use nym_crypto::asymmetric::x25519;
use nym_kkt_ciphersuite::{Ciphersuite, KEM, KEMKeyDigests};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::Arc;

pub use libcrux_psq::handshake::types::{DHKeyPair, DHPublicKey};
pub use nym_kkt::keys::KEMKeys;

/// Representation of a local Lewes Protocol peer
/// encapsulating all the known information and keys.
#[derive(Clone)]
pub struct LpLocalPeer {
    pub(crate) ciphersuite: Ciphersuite,

    /// Local x25519 keys (Noise static key)
    pub(crate) x25519: Arc<DHKeyPair>,

    /// Local KEM keys used for PSQ
    pub(crate) kem_keypairs: Option<KEMKeys>,
}

impl LpLocalPeer {
    pub fn new(ciphersuite: Ciphersuite, x25519: Arc<DHKeyPair>) -> Self {
        LpLocalPeer {
            ciphersuite,
            x25519,
            kem_keypairs: Default::default(),
        }
    }

    #[must_use]
    pub fn with_kem_keys(mut self, kem_keys: KEMKeys) -> Self {
        self.kem_keypairs = Some(kem_keys);
        self
    }

    pub fn x25519(&self) -> &Arc<DHKeyPair> {
        &self.x25519
    }

    /// Convert this `LpLocalPeer` into a valid `LpRemotePeer` that can be used within tests
    #[doc(hidden)]
    pub fn as_remote(&self) -> LpRemotePeer {
        let expected_kem_key_digests = self
            .kem_keypairs
            .as_ref()
            .map(|k| k.encapsulation_keys_digests())
            .unwrap_or_default();

        LpRemotePeer {
            x25519_public: self.x25519.pk,
            expected_kem_key_digests,
        }
    }

    pub fn ciphersuite(&self) -> Ciphersuite {
        self.ciphersuite
    }
}

impl Debug for LpLocalPeer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LpLocalPeer")
            .field("ciphersuite", &self.ciphersuite)
            .field("x25519", &self.x25519.pk)
            .field("kem_keypairs", &self.kem_keypairs)
            .finish()
    }
}

/// Representation of a remote Lewes Protocol peer
/// encapsulating all the known information and keys.
#[derive(Debug, Clone)]
pub struct LpRemotePeer {
    /// Remote X25519 public key (Noise static key)
    pub(crate) x25519_public: DHPublicKey,

    /// Expected digests of the remote's KEM key
    pub(crate) expected_kem_key_digests: BTreeMap<KEM, KEMKeyDigests>,
}

impl LpRemotePeer {
    pub fn new(x25519_public: x25519::PublicKey) -> Self {
        // TODO: make nicer conversion (without cloning) + error handling
        let responder_x25519_public_key = DHPublicKey::from_bytes(x25519_public.as_bytes());
        LpRemotePeer {
            x25519_public: responder_x25519_public_key,
            expected_kem_key_digests: Default::default(),
        }
    }

    pub fn x25519(&self) -> &DHPublicKey {
        &self.x25519_public
    }

    #[must_use]
    pub fn with_key_digests(
        mut self,
        expected_kem_key_digests: BTreeMap<KEM, KEMKeyDigests>,
    ) -> Self {
        self.expected_kem_key_digests = expected_kem_key_digests;
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

impl From<DHPublicKey> for LpRemotePeer {
    fn from(value: DHPublicKey) -> Self {
        LpRemotePeer {
            x25519_public: value,
            expected_kem_key_digests: Default::default(),
        }
    }
}

#[cfg(any(feature = "mock", test))]
pub fn mock_peer() -> LpLocalPeer {
    // use deterministic rng
    let mut rng = nym_test_utils::helpers::deterministic_rng_09();
    random_peer(&mut rng)
}

#[cfg(any(feature = "mock", test))]
pub fn random_peer<R: rand09::CryptoRng + rand09::RngCore>(rng: &mut R) -> LpLocalPeer {
    let x25519 = Arc::new(nym_kkt::key_utils::generate_lp_keypair_x25519(rng));

    LpLocalPeer {
        ciphersuite: Ciphersuite::default(),

        x25519,
        kem_keypairs: Some(KEMKeys::new(
            nym_kkt::key_utils::generate_keypair_mceliece(rng),
            nym_kkt::key_utils::generate_keypair_mlkem(rng),
        )),
    }
}

#[cfg(any(feature = "mock", test))]
pub fn mock_peers() -> (LpLocalPeer, LpLocalPeer) {
    let mut rng = nym_test_utils::helpers::deterministic_rng_09();

    (random_peer(&mut rng), random_peer(&mut rng))
}
