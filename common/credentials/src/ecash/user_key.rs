// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials_interface::KeyPairUser;
use nym_validator_client::nyxd::bip39;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

// changing this changes the ecash key of every client that derives it from its mnemonic;
// a new construction needs a new label version
const ECASH_USER_KEY_LABEL: &[u8] = b"nym-ecash-user-key-from-mnemonic-v1";

/// The ecash keypair that ticketbooks paid for with `mnemonic` are issued under.
///
/// It comes from the paying account and not from any mixnet client key, so refreshing or
/// resetting the client's keys leaves it unchanged.
pub fn ecash_keypair_from_mnemonic(mnemonic: &bip39::Mnemonic) -> KeyPairUser {
    let entropy = Zeroizing::new(mnemonic.to_entropy());

    let mut hasher = Sha256::new();
    hasher.update(ECASH_USER_KEY_LABEL);
    hasher.update(entropy.as_slice());
    let seed: Zeroizing<[u8; 32]> = Zeroizing::new(hasher.finalize().into());

    KeyPairUser::new_seeded(seed.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_credentials_interface::Base58;

    // fixed bytes rather than a generated mnemonic, so the input cannot move under the known answer
    const ENTROPY: [u8; 32] = [7; 32];

    // computed from the spec with an independent SHA-256, not from this implementation
    const EXPECTED_SEED: [u8; 32] = [
        0x23, 0x56, 0xd7, 0xca, 0x84, 0xd6, 0x6c, 0xa7, 0xf3, 0x73, 0x93, 0xbc, 0x82, 0x86, 0x41,
        0xa8, 0x97, 0x2b, 0xb2, 0xb0, 0x58, 0xe4, 0xee, 0xe5, 0x22, 0x0e, 0x2f, 0xbc, 0x77, 0x1b,
        0x15, 0xb6,
    ];

    // a changed output changes the ecash key of every self-funding client, so its new ticketbooks
    // are issued under a different key than its old ones. The public key was recorded from this
    // implementation, since it comes from the existing seeded ecash key generation
    #[test]
    fn derived_keypair_matches_its_known_answer() {
        let mnemonic = bip39::Mnemonic::from_entropy(&ENTROPY).unwrap();
        let keypair = ecash_keypair_from_mnemonic(&mnemonic);

        assert_eq!(keypair, KeyPairUser::new_seeded(EXPECTED_SEED));
        assert_eq!(
            keypair.public_key().to_bs58(),
            "722Qxm8HhEtRNuYvSSp7vovP3WB4vrr2UVJ4LKzXN1SKF2vhro3Ut1ExL3bGqNxGFX"
        );
    }
}
