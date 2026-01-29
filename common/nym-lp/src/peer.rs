// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use libcrux_kem::{MlKem768PrivateKey, MlKem768PublicKey};
use libcrux_psq::handshake::Responder;
use libcrux_psq::handshake::types::{DHKeyPair, DHPrivateKey, DHPublicKey};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kkt::ciphersuite::{
    Ciphersuite, DecapsulationKey, EncapsulationKey, KEM, KEMKeyDigests, SignatureScheme,
    SigningKeyDigests,
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
    pub(crate) ciphersuite: Arc<Ciphersuite>,
    /// Local Ed25519 keys for PSQ authentication
    pub(crate) ed25519: Arc<ed25519::KeyPair>,

    /// Local x25519 keys (Noise static key)
    pub(crate) x25519: Arc<DHKeyPair>,

    /// Local MlKem keypair used for PSQ
    pub(crate) mlkem: Option<Arc<(DecapsulationKey, EncapsulationKey)>>,

    /// Local McEliece keypair used for PSQ
    pub(crate) mceliece: Option<Arc<(DecapsulationKey, EncapsulationKey)>>,
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
            ciphersuite: Arc::new(ciphersuite),
            ed25519,
            x25519: Arc::new(initiator_x25519_keypair),
            mlkem: None,
            mceliece: None,
        }
    }

    pub fn with_mlkem_keypair(
        mut self,
        decapsulation_key: &MlKem768PrivateKey,
        encapsulation_key: &MlKem768PublicKey,
    ) -> Self {
        self.mlkem = Some(Arc::new((
            DecapsulationKey::MlKem768(decapsulation_key.clone()),
            EncapsulationKey::MlKem768(encapsulation_key.clone()),
        )));
        self
    }

    pub fn with_mceliece_keypair(
        mut self,
        decapsulation_key: libcrux_psq::classic_mceliece::SecretKey,
        encapsulation_key: libcrux_psq::classic_mceliece::PublicKey,
    ) -> Self {
        self.mceliece = Some(Arc::new((
            DecapsulationKey::McEliece(decapsulation_key),
            EncapsulationKey::McEliece(encapsulation_key),
        )));
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

        if let Some(mlkem_key) = &self.mlkem {
            expected_kem_key_digests.insert(
                KEM::MlKem768,
                nym_kkt::key_utils::produce_key_digests(&mlkem_key.1.encode()),
            );
        }

        if let Some(mceliece_key) = &self.mceliece {
            expected_kem_key_digests.insert(
                KEM::McEliece,
                nym_kkt::key_utils::produce_key_digests(&mceliece_key.1.encode()),
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
            .field("ed25519", &self.ed25519)
            .field("x25519", &self.x25519.pk)
            .field(
                "mlkem",
                &format!(
                    "mlkem_public_key: {}",
                    match &self.mlkem {
                        Some(keypair) => format!("{:?}", keypair.as_ref().1),
                        None => "None".to_string(),
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
}

#[cfg(test)]
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

#[cfg(test)]
pub fn random_peer<'a, R: rand::CryptoRng + rand::RngCore>(
    rng: &mut R,
    ciphersuite: Ciphersuite,
) -> LpLocalPeer {
    use nym_kkt::key_utils::{generate_keypair_mceliece, generate_keypair_mlkem};

    let ed25519 = Arc::new(ed25519::KeyPair::new(rng));

    let mut sk = [0u8; 32];
    rng.fill_bytes(&mut sk);

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

#[cfg(test)]
pub fn mock_peers(kem: KEM) -> (LpLocalPeer, LpLocalPeer) {
    println!("KEM: {:?}", kem);
    (mock_peer(kem), mock_peer(kem))
}
