// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "asymmetric")]
pub mod asymmetric;
pub mod bech32_address_validation;
#[cfg(feature = "hashing")]
pub mod crypto_hash;
#[cfg(feature = "hashing")]
pub mod hkdf;
#[cfg(feature = "hashing")]
pub mod hmac;
#[cfg(all(feature = "asymmetric", feature = "hashing", feature = "stream_cipher"))]
pub mod shared_key;
pub mod symmetric;

#[cfg(feature = "hashing")]
pub use digest::{Digest, OutputSizeUser};
#[cfg(any(feature = "hashing", feature = "stream_cipher", feature = "aead"))]
pub use generic_array;

// with the below my idea was to try to introduce having a single place of importing all hashing, encryption,
// etc. algorithms and import them elsewhere as needed via common/crypto
#[cfg(feature = "stream_cipher")]
pub use aes;
#[cfg(feature = "aead")]
pub use aes_gcm_siv::{Aes128GcmSiv, Aes256GcmSiv};
#[cfg(feature = "hashing")]
pub use blake3;
#[cfg(feature = "stream_cipher")]
pub use ctr;
