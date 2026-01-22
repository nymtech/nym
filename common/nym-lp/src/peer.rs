// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::{ed25519, x25519};
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

    /// Convert this `LpLocalPeer` into a valid `LpRemotePeer` that can be used within tests
    #[doc(hidden)]
    pub fn as_remote(&self) -> LpRemotePeer {
        let expected_kem_key_digest = match &self.kem_psq {
            None => Vec::new(),
            Some(kem_keys) => nym_kkt::key_utils::hash_key_bytes(
                &nym_kkt::ciphersuite::HashFunction::Blake3,
                nym_kkt::ciphersuite::DEFAULT_HASH_LEN,
                kem_keys.public_key().as_bytes(),
            ),
        };
        LpRemotePeer {
            ed25519_public: *self.ed25519.public_key(),
            x25519_public: *self.x25519.public_key(),
            expected_kem_key_digest,
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

    /// Expected digest of the remote's KEM key
    // TODO: this might have to be replaced by a HashMap<HashFunction, Vec<u8>> instead
    pub(crate) expected_kem_key_digest: Vec<u8>,
}

impl LpRemotePeer {
    pub fn new(ed25519_public: ed25519::PublicKey, x25519_public: x25519::PublicKey) -> Self {
        LpRemotePeer {
            ed25519_public,
            x25519_public,
            expected_kem_key_digest: vec![],
        }
    }

    pub fn ed25519(&self) -> ed25519::PublicKey {
        self.ed25519_public
    }

    pub fn x25519(&self) -> x25519::PublicKey {
        self.x25519_public
    }

    #[must_use]
    pub fn with_kem_key_digest(mut self, expected_kem_key_digest: Vec<u8>) -> Self {
        self.expected_kem_key_digest = expected_kem_key_digest;
        self
    }
}

#[cfg(test)]
pub fn mock_peer() -> LpLocalPeer {
    // use deterministic rng
    let mut rng = nym_test_utils::helpers::deterministic_rng();
    random_peer(&mut rng)
}

#[cfg(test)]
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

#[cfg(test)]
pub fn mock_peers() -> (LpLocalPeer, LpLocalPeer) {
    // use deterministic rng
    let mut rng = nym_test_utils::helpers::deterministic_rng();

    (random_peer(&mut rng), random_peer(&mut rng))
}
