// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Zeroize, ZeroizeOnDrop, Clone)]
pub struct StoredIssuedCredential {
    pub id: i64,

    pub serialization_revision: u8,
    pub credential_data: Vec<u8>,

    pub epoch_id: u32,
    pub expired: bool,
    pub consumed: bool,
}

pub struct StorableIssuedCredential<'a> {
    pub serialization_revision: u8,
    pub credential_data: &'a [u8],

    pub epoch_id: u32,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
pub struct CredentialUsage {
    pub credential_id: i64,
    pub gateway_id_bs58: String,
}

#[derive(Clone)]
pub struct CoinIndicesSignature {
    pub epoch_id: i64,
    pub signatures: String,
}
