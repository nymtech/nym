// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PaymentResponse {
    pub unyms_bought: u64,
}
