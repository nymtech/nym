// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use prost::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SphinxKeys {
    //
}
