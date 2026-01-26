// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use libcrux_kem::{MlKem768PrivateKey, MlKem768PublicKey};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kkt::ciphersuite::{DecapsulationKey, EncapsulationKey, HashFunction, KEM};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// Representation of a local Lewes Protocol peer
/// encapsulating all the known information and keys.
#[derive(Clone)]
pub struct LpLocalPeer {
    /// Local Ed25519 keys for PSQ authentication
    pub(crate) ed25519: Arc<ed25519::KeyPair>,

    /// Local x25519 keys (Noise static key)
    pub(crate) x25519: Arc<x25519::KeyPair>,

    /// Local KEM key used for PSQ (x25519: to deprecate)
    pub(crate) kem_psq: Option<Arc<x25519::KeyPair>>,

    /// Local MlKem keypair used for PSQ
    pub(crate) mlkem: Option<(Arc<DecapsulationKey>, Arc<EncapsulationKey>)>,

    /// Local McEliece keypair used for PSQ
    pub(crate) mceliece: Option<(Arc<DecapsulationKey>, Arc<EncapsulationKey>)>,
}

impl LpLocalPeer {
    pub fn new(ed25519: Arc<ed25519::KeyPair>, x25519: Arc<x25519::KeyPair>) -> Self {
        LpLocalPeer {
            ed25519,
            x25519,
            kem_psq: None,
            mlkem: None,
            mceliece: None,
        }
    }

    // #[must_use]
    pub fn with_kem_psq_key(mut self, key: Arc<x25519::KeyPair>) -> Self {
        self.kem_psq = Some(key);
        self
    }

    pub fn with_mlkem_keypair(
        mut self,
        decapsulation_key: &MlKem768PrivateKey,
        encapsulation_key: &MlKem768PublicKey,
    ) -> Self {
        self.mlkem = Some((
            Arc::new(DecapsulationKey::MlKem768(decapsulation_key.clone())),
            Arc::new(EncapsulationKey::MlKem768(encapsulation_key.clone())),
        ));
        self
    }

    pub fn with_mceliece_keypair(
        mut self,
        decapsulation_key: &MlKem768PrivateKey,
        encapsulation_key: &MlKem768PublicKey,
    ) -> Self {
        self.mlkem = Some((
            Arc::new(DecapsulationKey::MlKem768(decapsulation_key.clone())),
            Arc::new(EncapsulationKey::MlKem768(encapsulation_key.clone())),
        ));
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
        let expected_kem_key_digests = match &self.kem_psq {
            None => HashMap::new(),
            Some(kem_keys) => {
                let hashes =
                    nym_kkt::key_utils::produce_key_digests(kem_keys.public_key().as_bytes());

                let mut digests = HashMap::new();
                digests.insert(KEM::X25519, hashes);
                digests
            }
        };
        LpRemotePeer {
            ed25519_public: *self.ed25519.public_key(),
            x25519_public: *self.x25519.public_key(),
            expected_kem_key_digests,
        }
    }

    // this is only exposed in tests as ideally we should be storing the proper types to begin with
    #[cfg(test)]
    pub fn encapsulate_kem_key(&self) -> Option<nym_kkt::ciphersuite::EncapsulationKey> {
        let pk_bytes = self.kem_psq.as_ref()?.public_key().to_bytes();
        let libcrux_pk =
            libcrux_kem::PublicKey::decode(libcrux_kem::Algorithm::X25519, &pk_bytes).ok()?;

        Some(nym_kkt::ciphersuite::EncapsulationKey::X25519(libcrux_pk))
    }
}

impl Debug for LpLocalPeer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LpLocalPeer")
            .field("ed25519", &self.ed25519)
            .field("x25519", &self.x25519)
            .field("kem_psq", &self.kem_psq)
            .field(
                "mlkem",
                &format!(
                    "mlkem_public_key: {}",
                    match &self.mlkem {
                        Some(keypair) => format!("{:?}", keypair.1.as_ref()),
                        None => format!("None"),
                    }
                ),
            )
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
    pub(crate) x25519_public: x25519::PublicKey,

    /// Expected digest of the remote's KEM key
    pub(crate) expected_kem_key_digests: HashMap<KEM, HashMap<HashFunction, Vec<u8>>>,
}

impl LpRemotePeer {
    pub fn new(ed25519_public: ed25519::PublicKey, x25519_public: x25519::PublicKey) -> Self {
        LpRemotePeer {
            ed25519_public,
            x25519_public,
            expected_kem_key_digests: Default::default(),
        }
    }

    pub fn ed25519(&self) -> ed25519::PublicKey {
        self.ed25519_public
    }

    pub fn x25519(&self) -> x25519::PublicKey {
        self.x25519_public
    }

    #[must_use]
    pub fn with_kem_key_digests(
        mut self,
        expected_kem_key_digests: HashMap<KEM, HashMap<HashFunction, Vec<u8>>>,
    ) -> Self {
        self.expected_kem_key_digests = expected_kem_key_digests;
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
pub fn random_peer<'a, R: rand::CryptoRng + rand::RngCore>(rng: &mut R) -> LpLocalPeer {
    use nym_kkt::key_utils::generate_keypair_mlkem;

    let ed25519 = Arc::new(ed25519::KeyPair::new(rng));
    let x25519 = Arc::new(ed25519.to_x25519());
    let kem_psq = Some(x25519.clone());

    let mlkem_keypair = generate_keypair_mlkem(&mut rand09::rng());

    LpLocalPeer {
        ed25519,
        x25519,
        kem_psq,
        mlkem: None,
        mceliece: None,
    }
    .with_mlkem_keypair(&mlkem_keypair.0, &mlkem_keypair.1)
}

#[cfg(test)]
pub fn mock_peers() -> (LpLocalPeer, LpLocalPeer) {
    // use deterministic rng
    let mut rng = nym_test_utils::helpers::deterministic_rng();

    (random_peer(&mut rng), random_peer(&mut rng))
}
