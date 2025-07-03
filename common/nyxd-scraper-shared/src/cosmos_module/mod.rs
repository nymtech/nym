// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::message_registry::MessageRegistry;

pub mod message_registry;
mod modules;

pub trait CosmosModule {
    fn register_messages(&self, registry: &mut MessageRegistry);
}
