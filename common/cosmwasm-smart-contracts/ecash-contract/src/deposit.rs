// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::EcashContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{StdError, StdResult};

/// Sequential identifier assigned to every accepted deposit. Starts at 0 and
/// is never recycled.
pub type DepositId = u32;

/// Opaque on-chain record of a deposit: the depositor-claimed bs58-encoded
/// ed25519 identity public key. The contract does not verify control of the
/// corresponding private key.
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

    /// Decode a bs58-encoded ed25519 public key to its 32-byte raw form.
    /// Surfaces `MalformedEd25519Identity` on any bs58 / length failure.
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

    /// Decode this deposit's identity key to its 32-byte raw form for storage.
    pub fn to_bytes(&self) -> Result<[u8; 32], EcashContractError> {
        Self::get_ed25519_pubkey_bytes(&self.bs58_encoded_ed25519_pubkey)
    }

    /// Reconstruct a `Deposit` from a raw 32-byte ed25519 pubkey as stored
    /// under the `"deposit"` namespace.
    pub fn try_from_bytes(bytes: &[u8]) -> StdResult<Self> {
        if bytes.len() != 32 {
            return Err(StdError::generic_err("malformed deposit data"));
        }

        Ok(Deposit {
            bs58_encoded_ed25519_pubkey: Self::encode_pubkey_bytes(bytes),
        })
    }
}

/// Response shape for `GetLatestDeposit`. `deposit` is `None` on a freshly
/// deployed contract.
#[cw_serde]
#[derive(Default)]
pub struct LatestDepositResponse {
    pub deposit: Option<DepositData>,
}

/// Response shape for `GetDeposit { deposit_id }`. `deposit` is `None` when
/// the id has not yet been assigned (`id >= total_deposits_made`).
#[cw_serde]
pub struct DepositResponse {
    pub id: DepositId,

    pub deposit: Option<Deposit>,
}

/// `(deposit_id, deposit)` pair surfaced by the latest-deposit and paginated
/// deposit queries.
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

/// Page of deposits returned by `GetDepositsPaged`. `start_next_after` is the
/// id of the last returned entry; pass it as the next call's `start_after`.
#[cw_serde]
pub struct PagedDepositsResponse {
    pub deposits: Vec<DepositData>,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<DepositId>,
}
