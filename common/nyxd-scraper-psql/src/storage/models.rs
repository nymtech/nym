// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::FromRow;

#[derive(Debug, Clone, Eq, PartialEq, Hash, FromRow)]
pub struct Validator {
    pub consensus_address: String,
    pub consensus_pubkey: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct Block {
    pub height: i64,
    pub hash: String,
    pub num_txs: u32,
    pub total_gas: i64,
    pub proposer_address: String,
    pub timestamp: OffsetDateTime,
}

#[derive(Debug, Clone, FromRow)]
pub struct CommitSignature {
    pub height: i64,
    pub validator_address: String,
    pub voting_power: i64,
    pub proposer_priority: i64,
    pub timestamp: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "coin")]
pub struct DbCoin {
    pub amount: String,
    pub denom: String,
}

impl From<cosmrs::proto::cosmos::base::v1beta1::Coin> for DbCoin {
    fn from(coin: cosmrs::proto::cosmos::base::v1beta1::Coin) -> Self {
        Self {
            amount: coin.amount,
            denom: coin.denom,
        }
    }
}
