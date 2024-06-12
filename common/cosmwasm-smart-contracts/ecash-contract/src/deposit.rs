// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::EcashContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

pub type DepositId = u32;

#[cw_serde]
pub struct Deposit {
    pub info: String,

    pub amount: Uint128,

    pub bs58_encoded_ed25519: String,
}

impl Deposit {
    pub fn get_ed25519_pubkey_bytes(raw: &str) -> Result<[u8; 32], EcashContractError> {
        let mut ed25519_pubkey_bytes = [0u8; 32];
        bs58::decode(raw)
            .onto(&mut ed25519_pubkey_bytes)
            .map_err(|_| EcashContractError::MalformedEd25519Identity)?;

        Ok(ed25519_pubkey_bytes)
    }

    pub fn encode_pubkey_bytes(raw: &[u8]) -> String {
        bs58::encode(raw).into_string()
    }

    pub fn ed25519_pubkey_bytes(&self) -> Result<[u8; 32], EcashContractError> {
        Self::get_ed25519_pubkey_bytes(&self.bs58_encoded_ed25519)
    }
}

#[cw_serde]
pub struct DepositResponse {
    pub id: DepositId,

    pub deposit: Option<Deposit>,
}

#[cw_serde]
pub struct DepositData {
    pub id: DepositId,

    pub deposit: Deposit,
}

impl From<(DepositId, Deposit)> for DepositData {
    fn from((id, deposit): (DepositId, Deposit)) -> Self {
        DepositData { id, deposit }
    }
}

#[cw_serde]
pub struct PagedDepositsResponse {
    pub deposits: Vec<DepositData>,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<DepositId>,
}
