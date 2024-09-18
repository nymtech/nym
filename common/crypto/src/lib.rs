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
#[cfg(all(feature = "asymmetric", feature = "hashing", feature = "symmetric"))]
pub mod shared_key;
#[cfg(feature = "symmetric")]
pub mod symmetric;

#[cfg(feature = "hashing")]
pub use digest::{Digest, OutputSizeUser};
#[cfg(any(feature = "hashing", feature = "symmetric"))]
pub use generic_array;

// with the below my idea was to try to introduce having a single place of importing all hashing, encryption,
// etc. algorithms and import them elsewhere as needed via common/crypto
#[cfg(feature = "symmetric")]
pub use aes;
#[cfg(feature = "hashing")]
pub use blake3;
#[cfg(feature = "symmetric")]
pub use ctr;
