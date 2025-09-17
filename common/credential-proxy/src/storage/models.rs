// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::ed25519;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row, Type};
use strum_macros::{Display, EnumString};
use time::{Date, OffsetDateTime};
use zeroize::Zeroizing;

pub(crate) struct StorableEcashDeposit {
    pub(crate) deposit_id: u32,
    pub(crate) deposit_tx_hash: String,
    pub(crate) requested_on: OffsetDateTime,
    pub(crate) deposit_amount: String,
    pub(crate) ed25519_deposit_private_key: Zeroizing<[u8; ed25519::SECRET_KEY_LENGTH]>,
}

impl<'r> FromRow<'r, SqliteRow> for StorableEcashDeposit {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        let deposit_id = row.try_get("deposit_id")?;
        let deposit_tx_hash = row.try_get("deposit_tx_hash")?;
        let requested_on = row.try_get("requested_on")?;
        let deposit_amount = row.try_get("deposit_amount")?;
        let ed25519_deposit_private_key: Vec<u8> = row.try_get("ed25519_deposit_private_key")?;
        if ed25519_deposit_private_key.len() != ed25519::SECRET_KEY_LENGTH {
            return Err(sqlx::Error::decode(
                "stored ed25519_deposit_private_key has invalid length",
            ));
        }

        // SAFETY: we just checked the length is correct
        #[allow(clippy::unwrap_used)]
        let ed25519_deposit_private_key: [u8; ed25519::SECRET_KEY_LENGTH] =
            ed25519_deposit_private_key.try_into().unwrap();

        Ok(StorableEcashDeposit {
            deposit_id,
            deposit_tx_hash,
            requested_on,
            deposit_amount,
            ed25519_deposit_private_key: Zeroizing::new(ed25519_deposit_private_key),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, EnumString, Type, PartialEq, Display)]
#[sqlx(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum BlindedSharesStatus {
    Pending,
    Issued,
    Error,
}

#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct BlindedShares {
    pub id: i64,
    pub request_uuid: String,
    pub status: BlindedSharesStatus,
    pub device_id: String,
    pub credential_id: String,
    pub available_shares: i64,
    pub error_message: Option<String>,
    pub created: OffsetDateTime,
    pub updated: OffsetDateTime,
}

#[derive(FromRow)]
pub struct RawExpirationDateSignatures {
    pub serialised_signatures: Vec<u8>,
    pub serialization_revision: u8,
}

#[derive(FromRow)]
pub struct RawCoinIndexSignatures {
    #[allow(dead_code)]
    pub epoch_id: u32,
    pub serialised_signatures: Vec<u8>,
    pub serialization_revision: u8,
}

#[derive(FromRow)]
pub struct RawVerificationKey {
    #[allow(dead_code)]
    pub epoch_id: u32,
    pub serialised_key: Vec<u8>,
    pub serialization_revision: u8,
}

#[derive(FromRow)]
pub struct WalletShare {
    #[allow(dead_code)]
    pub corresponding_deposit: i64,
    pub node_id: i64,
    #[allow(dead_code)]
    pub created: OffsetDateTime,
    pub blinded_signature: Vec<u8>,
}

#[derive(FromRow)]
pub struct MinimalWalletShare {
    pub epoch_id: i64,
    pub expiration_date: Date,
    pub node_id: i64,
    pub blinded_signature: Vec<u8>,
}

impl From<MinimalWalletShare>
    for nym_credential_proxy_requests::api::v1::ticketbook::models::WalletShare
{
    fn from(value: MinimalWalletShare) -> Self {
        nym_credential_proxy_requests::api::v1::ticketbook::models::WalletShare {
            node_index: value.node_id as u64,
            bs58_encoded_share: bs58::encode(&value.blinded_signature).into_string(),
        }
    }
}
