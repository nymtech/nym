// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::error::CoconutError;
use nym_api_requests::coconut::BlindedSignatureResponse;
use nym_coconut::{Base58, BlindedSignature};
use std::fmt::Display;

pub struct EpochCredentials {
    pub epoch_id: u32,
    pub start_id: i64,
    pub total_issued: u32,
}

pub struct IssuedCredential {
    pub id: i64,
    pub epoch_id: u32,
    pub tx_hash: String,

    /// base58-encoded issued credential
    pub bs58_partial_credential: String,

    /// base58-encoded signature on the issued credential (and the attributes)
    pub bs58_signature: String,

    // i.e. "'attr1','attr2',..."
    pub joined_private_commitments: String,

    // i.e. "'attr1','attr2',..."
    pub joined_public_attributes: String,
}

impl TryFrom<IssuedCredential> for BlindedSignatureResponse {
    type Error = CoconutError;

    fn try_from(value: IssuedCredential) -> Result<Self, Self::Error> {
        Ok(BlindedSignatureResponse {
            blinded_signature: BlindedSignature::try_from_bs58(value.bs58_partial_credential)?,
        })
    }
}

impl IssuedCredential {
    // safety: this should only ever be called on sanitized data from the database,
    // thus the unwraps are fine (if somebody manually entered their db file and modified it, it's on them)
    // pub fn private_attribute_commitments(&self) -> Vec<Scalar>
    // pub fn public_attributes(&self)
}

pub fn join_attributes<I, M>(attrs: I) -> String
where
    I: IntoIterator<Item = M>,
    M: Display,
{
    // I could have used `attrs.into_iter().join(",")`,
    // but my IDE didn't like it (compiler was fine)
    itertools::Itertools::join(&mut attrs.into_iter(), ",")
}

pub fn split_attributes(attrs: &str) -> Vec<String> {
    attrs.split(',').map(|s| s.to_string()).collect()
}
