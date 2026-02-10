// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{ClientHelloData, LpError};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kkt::ciphersuite::{Ciphersuite, KEM, KEMKeyDigests, SignatureScheme, SigningKeyDigests};
use std::collections::HashMap;
use std::sync::Arc;

/// Representation of a local Lewes Protocol peer
/// encapsulating all the known information and keys.
#[derive(Debug, Clone)]
pub struct LpLocalPeer {
    /// Local Ed25519 keys for PSQ authentication
    pub(crate) ed25519: Arc<ed25519::KeyPair>,

    /// Local x25519 keys (Noise static key)
    pub(crate) x25519: Arc<x25519::KeyPair>,

    /// Local KEM key used for PSQ
    pub(crate) kem_psq: Option<Arc<x25519::KeyPair>>,
}

impl LpLocalPeer {
    pub fn new(ed25519: Arc<ed25519::KeyPair>, x25519: Arc<x25519::KeyPair>) -> Self {
        LpLocalPeer {
            ed25519,
            x25519,
            kem_psq: None,
        }
    }

    pub fn build_client_hello_data(&self, timestamp: u64) -> ClientHelloData {
        ClientHelloData::new_with_fresh_salt(
            *self.x25519().public_key(),
            *self.ed25519().public_key(),
            timestamp,
        )
    }

    #[must_use]
    pub fn with_kem_psq_key(mut self, key: Arc<x25519::KeyPair>) -> Self {
        self.kem_psq = Some(key);
        self
    }

    pub fn ed25519(&self) -> &Arc<ed25519::KeyPair> {
        &self.ed25519
    }

    pub fn x25519(&self) -> &Arc<x25519::KeyPair> {
        &self.x25519
    }

    /// Returns the reference to the KEM Public key of the peer (if available).
    pub fn get_kem_key_handle(&self) -> Result<&x25519::PublicKey, LpError> {
        self.kem_psq
            .as_ref()
            .map(|kp| kp.public_key())
            .ok_or(LpError::ResponderWithMissingKEMKey)
    }

    /// Convert this `LpLocalPeer` into a valid `LpRemotePeer` that can be used within tests
    #[doc(hidden)]
    pub fn as_remote(&self) -> LpRemotePeer {
        let expected_kem_key_digests = match &self.kem_psq {
            None => HashMap::new(),
            Some(kem_keys) => {
                let mut digests = HashMap::new();
                digests.insert(
                    KEM::X25519,
                    nym_kkt::key_utils::produce_key_digests(kem_keys.public_key().as_bytes()),
                );
                digests
            }
        };

        let mut expected_signing_key_digests = HashMap::new();
        expected_signing_key_digests.insert(
            SignatureScheme::Ed25519,
            nym_kkt::key_utils::produce_key_digests(self.ed25519.public_key().as_bytes()),
        );

        LpRemotePeer {
            ed25519_public: *self.ed25519.public_key(),
            x25519_public: *self.x25519.public_key(),
            expected_kem_key_digests,
            expected_signing_key_digests,
        }
    }

    // this is only exposed in tests as ideally we should be storing the proper types to begin with
    #[cfg(test)]
    pub fn encapsulate_kem_key(&self) -> Option<nym_kkt::ciphersuite::EncapsulationKey<'_>> {
        let pk_bytes = self.kem_psq.as_ref()?.public_key().to_bytes();
        let libcrux_pk =
            libcrux_kem::PublicKey::decode(libcrux_kem::Algorithm::X25519, &pk_bytes).ok()?;

        Some(nym_kkt::ciphersuite::EncapsulationKey::X25519(libcrux_pk))
    }
}

/// Representation of a remote Lewes Protocol peer
/// encapsulating all the known information and keys.
#[derive(Debug, Clone)]
pub struct LpRemotePeer {
    /// Remote Ed25519 public key for PSQ authentication
    pub(crate) ed25519_public: ed25519::PublicKey,

    /// Remote X25519 public key (Noise static key)
    pub(crate) x25519_public: x25519::PublicKey,

    /// Expected digests of the remote's KEM key
    pub(crate) expected_kem_key_digests: HashMap<KEM, KEMKeyDigests>,

    /// Expected digests of the remote's signing key
    pub(crate) expected_signing_key_digests: HashMap<SignatureScheme, SigningKeyDigests>,
}

impl LpRemotePeer {
    pub fn new(ed25519_public: ed25519::PublicKey, x25519_public: x25519::PublicKey) -> Self {
        LpRemotePeer {
            ed25519_public,
            x25519_public,
            expected_kem_key_digests: Default::default(),
            expected_signing_key_digests: Default::default(),
        }
    }

    pub fn ed25519(&self) -> ed25519::PublicKey {
        self.ed25519_public
    }

    pub fn x25519(&self) -> x25519::PublicKey {
        self.x25519_public
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
pub fn random_peer<R: rand::CryptoRng + rand::RngCore>(rng: &mut R) -> LpLocalPeer {
    let ed25519 = Arc::new(ed25519::KeyPair::new(rng));
    let x25519 = Arc::new(ed25519.to_x25519());
    let kem_psq = Some(x25519.clone());

    LpLocalPeer {
        ed25519,
        x25519,
        kem_psq,
    }
}

#[cfg(any(feature = "mock", test))]
pub fn mock_peers() -> (LpLocalPeer, LpLocalPeer) {
    // use deterministic rng
    let mut rng = nym_test_utils::helpers::deterministic_rng();

    (random_peer(&mut rng), random_peer(&mut rng))
}
