// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{CosmosModule, MessageRegistry};

pub(crate) struct IbcTransferV2;

impl CosmosModule for IbcTransferV2 {
    fn register_messages(&self, _registry: &mut MessageRegistry) {}
}
