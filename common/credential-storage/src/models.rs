// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub struct CoconutCredential {
    #[allow(dead_code)]
    pub id: i64,
    pub voucher_value: String,
    pub voucher_info: String,
    pub serial_number: String,
    pub binding_number: String,
    pub signature: String,
}

pub struct ERC20Credential {
    #[allow(dead_code)]
    pub id: i64,
    pub public_key: String,
    pub private_key: String,
    pub consumed: bool,
}
