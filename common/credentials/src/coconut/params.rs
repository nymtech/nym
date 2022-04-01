// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::aes::Aes128;
use crypto::blake3;
use crypto::ctr;

type Aes128Ctr = ctr::Ctr64LE<Aes128>;

/// Hashing algorithm used during hkdf for ephemeral shared key generation per blinded signature
/// response encryption.
pub type ValidatorApiCredentialHkdfAlgorithm = blake3::Hasher;

/// Encryption algorithm used for end-to-end encryption of blinded signature response
pub type ValidatorApiCredentialEncryptionAlgorithm = Aes128Ctr;
