// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::EcashContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{StdError, StdResult};

pub type DepositId = u32;

#[cw_serde]
pub struct Deposit {
    pub bs58_encoded_ed25519_pubkey: String,
}

impl Deposit {
    pub fn new(bs58_encoded_ed25519_pubkey: String) -> Self {
        Deposit {
            bs58_encoded_ed25519_pubkey,
        }
    }

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

    pub fn to_bytes(&self) -> Result<[u8; 32], EcashContractError> {
        Self::get_ed25519_pubkey_bytes(&self.bs58_encoded_ed25519_pubkey)
    }

    pub fn try_from_bytes(bytes: &[u8]) -> StdResult<Self> {
        if bytes.len() != 32 {
            return Err(StdError::generic_err("malformed deposit data"));
        }

        Ok(Deposit {
            bs58_encoded_ed25519_pubkey: Self::encode_pubkey_bytes(bytes),
        })
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
