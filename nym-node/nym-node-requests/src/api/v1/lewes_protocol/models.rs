// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct LewesProtocol {
    /// Helper field that specifies whether the LP listener(s) is enabled on this node.
    /// It is directly controlled by the node's role (i.e. it is enabled if it supports 'entry' mode)
    pub enabled: bool,

    /// LP TCP control address (default: 41264) for establishing LP sessions
    pub control_port: u16,

    /// LP UDP data address (default: 51264) for Sphinx packets wrapped in LP
    pub data_port: u16,
}
