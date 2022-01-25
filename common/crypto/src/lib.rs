// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod asymmetric;
pub mod bech32_address_validation;
pub mod crypto_hash;
pub mod hkdf;
pub mod hmac;
pub mod shared_key;
pub mod symmetric;

pub use digest::Digest;
pub use generic_array;

// with the below my idea was to try to introduce having a single place of importing all hashing, encryption,
// etc. algorithms and import them elsewhere as needed via common/crypto
pub use aes;
pub use blake3;

// TODO: this function uses all three modules: asymmetric crypto, symmetric crypto and derives key...,
// so I don't know where to put it...
