// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::api::v1::node::models::HostInformation;
use crate::error::Error;
use nym_crypto::asymmetric::identity;
use serde::Serialize;
use std::ops::Deref;

pub mod v1;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", aliases(SignedHostInformation = SignedData<HostInformation>))]
pub struct SignedData<T> {
    // #[serde(flatten)]
    pub data: T,
    pub signature: String,
}

impl<T> SignedData<T> {
    pub fn new(data: T, key: &identity::PrivateKey) -> Result<Self, Error>
    where
        T: Serialize,
    {
        let plaintext = serde_json::to_string(&data)?;
        let signature = key.sign(plaintext).to_base58_string();
        Ok(SignedData { data, signature })
    }

    pub fn verify(&self, key: &identity::PublicKey) -> bool
    where
        T: Serialize,
    {
        let Ok(plaintext) = serde_json::to_string(&self.data) else {
            return false;
        };
        let Ok(signature) = identity::Signature::from_base58_string(&self.signature) else {
            return false;
        };

        key.verify(plaintext, &signature).is_ok()
    }
}

impl<T> Deref for SignedData<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
