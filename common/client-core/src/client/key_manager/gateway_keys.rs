// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core_gateways_storage::GenerationId;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_crypto::blake3;
use nym_crypto::hkdf::SimpleHkdf;
use zeroize::Zeroizing;

// changing any of these changes every derived key; a new construction needs a new salt version
const GATEWAY_KEYS_HKDF_SALT: &[u8] = b"nym-client-gateway-keys-v1";
const IDENTITY_KEY_LABEL: &[u8] = b"identity";
const DIFFIE_HELLMAN_KEY_LABEL: &[u8] = b"diffie-hellman";

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
        let secret = self.expand::<{ ed25519::SECRET_KEY_LENGTH }>(IDENTITY_KEY_LABEL);

        // SAFETY: the secret has exactly the length `from_bytes` requires
        #[allow(clippy::expect_used)]
        let private_key = ed25519::PrivateKey::from_bytes(secret.as_slice())
            .expect("the derived secret has the length of an ed25519 private key");
        private_key.into()
    }

    /// The x25519 keypair in the client's address at the gateway.
    pub(super) fn diffie_hellman_keypair(&self) -> x25519::KeyPair {
        let secret = self.expand::<{ x25519::PRIVATE_KEY_SIZE }>(DIFFIE_HELLMAN_KEY_LABEL);

        // SAFETY: the secret has exactly the length `from_bytes` requires
        #[allow(clippy::expect_used)]
        let private_key = x25519::PrivateKey::from_bytes(secret.as_slice())
            .expect("the derived secret has the length of an x25519 private key");
        private_key.into()
    }

    fn expand<const N: usize>(&self, label: &[u8]) -> Zeroizing<[u8; N]> {
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

        let mut secret = Zeroizing::new([0; N]);
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
        let derivation =
            GatewayKeyDerivation::new(&private_key(1), public_key(2), GenerationId::INITIAL);

        assert_eq!(
            derivation
                .identity_keypair()
                .public_key()
                .to_base58_string(),
            "G6NNAq2WQigmFSu4QSDEyG6ddSWSKZjTMed9fJ1Pafwg"
        );
        assert_eq!(
            derivation
                .diffie_hellman_keypair()
                .public_key()
                .to_base58_string(),
            "EZQ4S5q6FdSsb556N6eNHxmvdxmMSh6trdrYpLpkPCTK"
        );
    }

    #[test]
    fn the_gateway_changes_both_keys() {
        let first =
            GatewayKeyDerivation::new(&private_key(1), public_key(2), GenerationId::INITIAL);
        let second =
            GatewayKeyDerivation::new(&private_key(1), public_key(3), GenerationId::INITIAL);

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
        let first =
            GatewayKeyDerivation::new(&private_key(1), public_key(2), GenerationId::INITIAL);
        let second = GatewayKeyDerivation::new(
            &private_key(1),
            public_key(2),
            GenerationId::new_random(&mut deterministic_rng()),
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
        let derivation =
            GatewayKeyDerivation::new(&private_key(1), public_key(2), GenerationId::INITIAL);

        assert_ne!(
            *derivation.expand::<32>(IDENTITY_KEY_LABEL),
            *derivation.expand::<32>(DIFFIE_HELLMAN_KEY_LABEL)
        );
    }
}
