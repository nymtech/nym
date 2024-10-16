// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use std::convert::Into;
use strum_macros::{Display, EnumString};
use time::{Date, OffsetDateTime};

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

pub struct FullBlindedShares {
    pub status: BlindedShares,
    pub shares: (),
}

#[derive(FromRow)]
pub struct RawExpirationDateSignatures {
    #[allow(dead_code)]
    pub epoch_id: u32,
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
