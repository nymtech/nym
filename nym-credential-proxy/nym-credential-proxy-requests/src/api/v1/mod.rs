// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// pub mod bandwidth_voucher;
// pub mod freepass;
pub mod ticketbook;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ErrorResponse {
    pub uuid: Option<Uuid>,
    pub message: String,
}
