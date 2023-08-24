// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[allow(unused)]
pub(crate) struct Payment {
    pub(crate) id: i64,
    pub(crate) serial_number: String,
    pub(crate) unyms_bought: i64,
    pub(crate) paid: bool,
}

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PaymentResponse {
    pub unyms_bought: u64,
}
