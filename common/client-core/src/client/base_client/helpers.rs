// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ClientCoreError;
use crate::{client::replies::reply_storage, config::DebugConfig};
use nym_task::{ShutdownManager, ShutdownToken, ShutdownTracker};

pub fn setup_empty_reply_surb_backend(debug_config: &DebugConfig) -> reply_storage::Empty {
    reply_storage::Empty {
        min_surb_threshold: debug_config
            .reply_surbs
            .minimum_reply_surb_storage_threshold,
        max_surb_threshold: debug_config
            .reply_surbs
            .maximum_reply_surb_storage_threshold,
    }
}

// old 'TaskHandle'
pub(crate) enum ShutdownHelper {
    Internal(ShutdownManager),
    External(ShutdownTracker),
}

fn new_shutdown_manager() -> Result<ShutdownManager, ClientCoreError> {
    cfg_if::cfg_if! {
        if #[cfg(not(target_arch = "wasm32"))] {
            Ok(ShutdownManager::new_without_signals().with_default_shutdown_signals()?.with_cancel_on_panic())
        } else {
            Ok(ShutdownManager::new())
        }
    }
}

impl ShutdownHelper {
    pub(crate) fn new(shutdown_tracker: Option<ShutdownTracker>) -> Result<Self, ClientCoreError> {
        match shutdown_tracker {
            None => Ok(ShutdownHelper::Internal(new_shutdown_manager()?)),
            Some(shutdown_tracker) => Ok(ShutdownHelper::External(shutdown_tracker)),
        }
    }

    pub(crate) fn into_internal(self) -> Option<ShutdownManager> {
        match self {
            ShutdownHelper::Internal(manager) => Some(manager),
            ShutdownHelper::External(_) => None,
        }
    }

    pub(crate) fn shutdown_token(&self) -> ShutdownToken {
        match self {
            ShutdownHelper::External(shutdown) => shutdown.clone_shutdown_token(),
            ShutdownHelper::Internal(shutdown) => shutdown.clone_shutdown_token(),
        }
    }

    pub(crate) fn tracker(&self) -> &ShutdownTracker {
        match self {
            ShutdownHelper::External(shutdown) => shutdown,
            ShutdownHelper::Internal(shutdown) => shutdown.shutdown_tracker(),
        }
    }
}
