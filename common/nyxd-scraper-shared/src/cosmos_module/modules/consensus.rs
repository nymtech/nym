// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::message_registry::MessageRegistry;
use crate::cosmos_module::CosmosModule;

pub(crate) struct Consensus;

impl CosmosModule for Consensus {
    fn register_messages(&self, _registry: &mut MessageRegistry) {}
}
