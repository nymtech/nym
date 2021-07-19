// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub trait PemStorableKey: Sized {
    type Error: std::error::Error;
    fn pem_type() -> &'static str;
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error>;
}

pub trait PemStorableKeyPair {
    type PrivatePemKey: PemStorableKey;
    type PublicPemKey: PemStorableKey;

    fn private_key(&self) -> &Self::PrivatePemKey;
    fn public_key(&self) -> &Self::PublicPemKey;
    fn from_keys(private_key: Self::PrivatePemKey, public_key: Self::PublicPemKey) -> Self;
}
