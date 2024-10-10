// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// pub mod bandwidth_voucher;
// pub mod freepass;
pub mod ticketbook;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ErrorResponse {
    #[cfg_attr(feature = "openapi",schema(value_type = Option<String>, example = "c48f9ce3-a1e9-4886-8000-13f290f34501"))]
    pub uuid: Option<Uuid>,
    pub message: String,
}
