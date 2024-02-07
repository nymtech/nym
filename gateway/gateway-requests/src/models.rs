// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::GatewayRequestsError;
use nym_credentials::coconut::bandwidth::CredentialSpendingData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialSpendingWithEpoch {
    /// The cryptographic material required for spending the underlying credential.
    pub data: CredentialSpendingData,

    /// The (DKG) epoch id under which the credential has been issued so that the verifier
    /// could use correct verification key for validation.
    pub epoch_id: u64,
}

impl CredentialSpendingWithEpoch {
    pub fn new(data: CredentialSpendingData, epoch_id: u64) -> Self {
        CredentialSpendingWithEpoch { data, epoch_id }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    pub fn try_from_bytes(raw: &[u8]) -> Result<Self, GatewayRequestsError> {
        todo!()
    }
}
