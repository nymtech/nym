// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(Clone)]
pub struct CoconutCredential {
    #[allow(dead_code)]
    pub id: i64,
    pub voucher_value: String,
    pub voucher_info: String,
    pub serial_number: String,
    pub binding_number: String,
    pub signature: String,
    pub epoch_id: String,
    pub consumed: bool,
}

#[derive(Clone)]
pub struct EcashCredential {
    #[allow(dead_code)]
    pub id: i64,
    pub voucher_info: String,
    pub wallet: String,
    pub secret_key: String,
    pub epoch_id: String,
    pub consumed: bool,
}
