// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use zeroize::{Zeroize, ZeroizeOnDrop};

// #[derive(Clone)]
// pub struct CoconutCredential {
//     #[allow(dead_code)]
//     pub id: i64,
//     pub voucher_value: String,
//     pub voucher_info: String,
//     pub serial_number: String,
//     pub binding_number: String,
//     pub signature: String,
//     pub epoch_id: String,
//     pub consumed: bool,
// }

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Zeroize, ZeroizeOnDrop, Clone)]
pub struct StoredIssuedCredential {
    pub id: i64,

    pub serialization_revision: u8,
    pub credential_data: Vec<u8>,
    pub credential_type: String,

    pub epoch_id: u32,
    pub consumed: bool,
}

pub struct StorableIssuedCredential<'a> {
    pub serialization_revision: u8,
    pub credential_data: &'a [u8],
    pub credential_type: String,

    pub epoch_id: u32,
}
