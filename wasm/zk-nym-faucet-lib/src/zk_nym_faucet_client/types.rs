// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// just copied over from dot com repo

use nym_coconut::BlindSignRequest;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use tsify::Tsify;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BandwidthVoucherRequest {
    /// base58 encoded blind sign request
    pub blind_sign_request: BlindSignRequest,
}

#[derive(Tsify, Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct BandwidthVoucherResponse {
    pub epoch_id: u64,
    pub shares: Vec<CredentialShare>,
}

#[derive(Tsify, Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct PartialVerificationKeysResponse {
    pub epoch_id: u64,
    pub keys: Vec<PartialVerificationKey>,
}

#[derive(Tsify, Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct CurrentEpochResponse {
    pub epoch_id: u64,
}

#[derive(Tsify, Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct CredentialShare {
    pub node_index: u64,
    pub bs58_encoded_share: String,
}

#[derive(Tsify, Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct PartialVerificationKey {
    pub node_index: u64,
    pub bs58_encoded_key: String,
}

#[derive(Tsify, Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct MasterVerificationKeyResponse {
    pub epoch_id: u64,
    pub bs58_encoded_key: String,
}

#[derive(Tsify, Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct DepositResponse {
    pub current_deposit_amount: u128,
    pub current_deposit_denom: String,
}

#[derive(Tsify, Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct AttributesResponse {
    pub credential_type_string: String,
    pub credential_amount_string: String,
    pub credential_amount_denom: String,

    pub bs58_prehashed_type: String,
    pub bs58_prehashed_amount: String,
}

#[derive(Tsify, Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub uuid: Option<Uuid>,
    pub message: String,
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(uuid) = self.uuid {
            write!(f, ". request uuid: {uuid}")?
        }
        Ok(())
    }
}
