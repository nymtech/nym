// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::aes::Aes128;
use nym_crypto::blake3;
use nym_crypto::ctr;

type Aes128Ctr = ctr::Ctr64LE<Aes128>;

/// Hashing algorithm used during hkdf for ephemeral shared key generation per blinded signature
/// response encryption.
pub type NymApiCredentialHkdfAlgorithm = blake3::Hasher;

/// Encryption algorithm used for end-to-end encryption of blinded signature response
pub type NymApiCredentialEncryptionAlgorithm = Aes128Ctr;
