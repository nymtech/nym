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
