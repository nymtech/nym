// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DeprecatedRouteResponse<T> {
    deprecated: bool,
    #[serde(flatten)]
    response: T,
}

// Note: this is a very dangerous method to call as the same identity in the past might have
// referred to a completely different node id!
fn mixnode_identity_to_current_node_id() {}
