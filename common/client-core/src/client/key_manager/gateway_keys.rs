// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core_gateways_storage::GenerationId;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_crypto::blake3;
use nym_crypto::hkdf::SimpleHkdf;
use nym_sphinx::acknowledgements::AckKey;
use std::sync::Arc;
use zeroize::Zeroizing;

// changing any of these changes every derived key; a new construction needs a new salt version
const GATEWAY_KEYS_HKDF_SALT: &[u8] = b"nym-client-gateway-keys-v1";
const IDENTITY_KEY_LABEL: &[u8] = b"identity";
const DIFFIE_HELLMAN_KEY_LABEL: &[u8] = b"diffie-hellman";

/// The keys a client uses with one gateway, resolved from the root keys and never stored.
pub struct GatewayClientKeys {
    pub(super) identity_keypair: Arc<ed25519::KeyPair>,
    pub(super) encryption_keypair: Arc<x25519::KeyPair>,
    pub(super) ack_key: Arc<AckKey>,
}

impl GatewayClientKeys {
    pub fn identity_keypair(&self) -> Arc<ed25519::KeyPair> {
        self.identity_keypair.clone()
    }

    pub fn encryption_keypair(&self) -> Arc<x25519::KeyPair> {
        self.encryption_keypair.clone()
    }

    pub fn ack_key(&self) -> Arc<AckKey> {
        self.ack_key.clone()
    }
}

/// Derives the keys a client uses with one gateway at one generation.
pub(super) struct GatewayKeyDerivation {
    pseudorandom_key: Zeroizing<[u8; blake3::OUT_LEN]>,
    gateway_id: ed25519::PublicKey,
    generation_id: GenerationId,
}

impl GatewayKeyDerivation {
    pub(super) fn new(
        root_identity: &ed25519::PrivateKey,
        gateway_id: ed25519::PublicKey,
        generation_id: GenerationId,
    ) -> Self {
        let input_key_material = Zeroizing::new(root_identity.to_bytes());
        let (pseudorandom_key, _) = SimpleHkdf::<blake3::Hasher>::extract(
            Some(GATEWAY_KEYS_HKDF_SALT),
            input_key_material.as_slice(),
        );

        GatewayKeyDerivation {
            pseudorandom_key: Zeroizing::new(pseudorandom_key.into()),
            gateway_id,
            generation_id,
        }
    }

    /// The identity keypair the client presents to the gateway.
    pub(super) fn identity_keypair(&self) -> ed25519::KeyPair {
        let secret = self.expand(IDENTITY_KEY_LABEL);

        // SAFETY: the secret has exactly the length `from_bytes` requires
        #[allow(clippy::expect_used)]
        let private_key = ed25519::PrivateKey::from_bytes(secret.as_slice())
            .expect("the derived secret has the length of an ed25519 private key");
        private_key.into()
    }

    /// The x25519 keypair in the client's address at the gateway.
    pub(super) fn diffie_hellman_keypair(&self) -> x25519::KeyPair {
        let secret = self.expand(DIFFIE_HELLMAN_KEY_LABEL);

        // SAFETY: the secret has exactly the length `from_bytes` requires
        #[allow(clippy::expect_used)]
        let private_key = x25519::PrivateKey::from_bytes(secret.as_slice())
            .expect("the derived secret has the length of an x25519 private key");
        private_key.into()
    }

    /// 32 bytes: the size of both an ed25519 and an x25519 private key.
    fn expand(&self, label: &[u8]) -> Zeroizing<[u8; 32]> {
        // SAFETY: the pseudorandom key is a full BLAKE3 output, the length `from_prk` requires
        #[allow(clippy::expect_used)]
        let hkdf = SimpleHkdf::<blake3::Hasher>::from_prk(self.pseudorandom_key.as_slice())
            .expect("the pseudorandom key has the length of a BLAKE3 output");
        let info = [
            label,
            self.gateway_id.as_bytes().as_slice(),
            self.generation_id.as_bytes().as_slice(),
        ]
        .concat();

        let mut secret = Zeroizing::new([0; 32]);
        // SAFETY: a single key is far below HKDF's limit of 255 hash outputs
        #[allow(clippy::expect_used)]
        hkdf.expand(&info, secret.as_mut_slice())
            .expect("a single key is a valid HKDF-BLAKE3 output length");
        secret
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::key_manager::ClientKeys;
    use nym_test_utils::helpers::deterministic_rng;

    // fixed bytes rather than a seeded generator, so the inputs cannot move under the known answer
    fn private_key(byte: u8) -> ed25519::PrivateKey {
        ed25519::PrivateKey::from_bytes(&[byte; ed25519::SECRET_KEY_LENGTH]).unwrap()
    }

    fn public_key(byte: u8) -> ed25519::PublicKey {
        ed25519::PublicKey::from(&private_key(byte))
    }

    // a changed output gives every derived registration a new identity, which strands its address
    // and the bandwidth credited to it at the gateway. The values were computed from the spec with an
    // independent HKDF-BLAKE3, Ed25519 and X25519, not from this implementation
    #[test]
    fn derived_keys_match_their_known_answer() {
        let generation_id = GenerationId::new_random(&mut deterministic_rng());
        // the known answer was computed for this id, so a changed generator fails here, not below
        assert_eq!(
            generation_id.as_bytes(),
            &[
                152, 25, 31, 70, 229, 131, 2, 22, 68, 84, 54, 151, 136, 3, 105, 122
            ]
        );

        let derivation = GatewayKeyDerivation::new(&private_key(1), public_key(2), generation_id);

        assert_eq!(
            derivation
                .identity_keypair()
                .public_key()
                .to_base58_string(),
            "3ER3EXxhWNfUcwvFJGgZrLEzLnty2LK65VicWaGk15P3"
        );
        assert_eq!(
            derivation
                .diffie_hellman_keypair()
                .public_key()
                .to_base58_string(),
            "6gR4medsGLdu8KKCytoZdZsiDwuPMYkYZcUUgL9RbDVt"
        );
    }

    #[test]
    fn the_gateway_changes_both_keys() {
        let generation_id = GenerationId::new_random(&mut deterministic_rng());
        let first = GatewayKeyDerivation::new(&private_key(1), public_key(2), generation_id);
        let second = GatewayKeyDerivation::new(&private_key(1), public_key(3), generation_id);

        assert_ne!(
            first.identity_keypair().public_key(),
            second.identity_keypair().public_key()
        );
        assert_ne!(
            first.diffie_hellman_keypair().public_key(),
            second.diffie_hellman_keypair().public_key()
        );
    }

    #[test]
    fn the_generation_id_changes_both_keys() {
        let mut rng = deterministic_rng();
        let first = GatewayKeyDerivation::new(
            &private_key(1),
            public_key(2),
            GenerationId::new_random(&mut rng),
        );
        let second = GatewayKeyDerivation::new(
            &private_key(1),
            public_key(2),
            GenerationId::new_random(&mut rng),
        );

        assert_ne!(
            first.identity_keypair().public_key(),
            second.identity_keypair().public_key()
        );
        assert_ne!(
            first.diffie_hellman_keypair().public_key(),
            second.diffie_hellman_keypair().public_key()
        );
    }

    #[test]
    fn the_key_type_changes_the_derived_secret() {
        let derivation = GatewayKeyDerivation::new(
            &private_key(1),
            public_key(2),
            GenerationId::new_random(&mut deterministic_rng()),
        );

        assert_ne!(
            *derivation.expand(IDENTITY_KEY_LABEL),
            *derivation.expand(DIFFIE_HELLMAN_KEY_LABEL)
        );
    }

    #[test]
    fn a_generation_resolves_to_the_derived_keys_and_the_root_ack_key() {
        let mut rng = deterministic_rng();
        let root = ClientKeys::generate_new(&mut rng);
        let generation_id = GenerationId::new_random(&mut rng);

        let keys = root.gateway_keys(public_key(2), Some(generation_id));
        let derivation = GatewayKeyDerivation::new(
            root.identity_keypair().private_key(),
            public_key(2),
            generation_id,
        );

        assert_eq!(
            keys.identity_keypair().public_key(),
            derivation.identity_keypair().public_key()
        );
        assert_eq!(
            keys.encryption_keypair().public_key(),
            derivation.diffie_hellman_keypair().public_key()
        );
        assert!(Arc::ptr_eq(&keys.ack_key(), &root.ack_key()));
    }
}
