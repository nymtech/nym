// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use nym_client_core_config_types::DebugConfig;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp_data::fragmentation::reconstruction::MessageReconstructor;
use nym_task::ShutdownToken;

/// Shared state for LP data plane
pub struct SharedLpDataState {
    pub(crate) config: DebugConfig,

    pub(crate) encryption_keys: Arc<x25519::KeyPair>,
    pub(crate) identity_keys: Arc<ed25519::KeyPair>,

    pub(crate) message_reconstructor: MessageReconstructor,

    pub(crate) shutdown_token: ShutdownToken,
}

impl SharedLpDataState {
    pub(crate) fn new(
        config: DebugConfig,
        encryption_keys: Arc<x25519::KeyPair>,
        identity_keys: Arc<ed25519::KeyPair>,
        shutdown_token: ShutdownToken,
    ) -> Self {
        SharedLpDataState {
            config,
            encryption_keys,
            identity_keys,
            message_reconstructor: Default::default(),
            shutdown_token,
        }
    }
}
