// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ClientCoreError;
use crate::{client::replies::reply_storage, config::DebugConfig};
use nym_task::{ShutdownManager, ShutdownToken};

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
    External(ShutdownToken),
}

fn new_shutdown_manager() -> Result<ShutdownManager, ClientCoreError> {
    cfg_if::cfg_if! {
        if #[cfg(not(target_arch = "wasm32"))] {
            Ok(ShutdownManager::new().with_default_shutdown_signals()?)
        } else {
            Ok(ShutdownManager::new())
        }
    }
}

impl ShutdownHelper {
    pub(crate) fn new(shutdown_token: Option<ShutdownToken>) -> Result<Self, ClientCoreError> {
        match shutdown_token {
            None => Ok(ShutdownHelper::Internal(new_shutdown_manager()?)),
            Some(shutdown_token) => Ok(ShutdownHelper::External(shutdown_token)),
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
            ShutdownHelper::External(shutdown) => shutdown.clone(),
            ShutdownHelper::Internal(shutdown) => shutdown.clone_shutdown_token(),
        }
    }
}
