// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

use crate::ShutdownManager;
use std::sync::RwLock;
use std::sync::{Arc, LazyLock};

/// Global registry that manages ShutdownManagers transparently.
/// This allows SDK components to get automatic task management without
/// exposing the complexity to end users.
pub(crate) struct RuntimeRegistry {
    // For SDK clients: auto-created manager without signal handling
    sdk_manager: RwLock<Option<Arc<ShutdownManager>>>,
}

#[derive(Debug, Error)]
pub enum RegistryAccessError {
    #[error("the runtime registry is poisoned")]
    Poisoned,
}

impl RuntimeRegistry {
    /// Get or create a ShutdownManager for SDK use.
    /// This manager doesn't listen to OS signals, making it suitable for library use.
    pub(crate) fn get_or_create_sdk() -> Result<Arc<ShutdownManager>, RegistryAccessError> {
        let guard = REGISTRY
            .sdk_manager
            .read()
            .map_err(|_| RegistryAccessError::Poisoned)?;
        if let Some(manager) = guard.as_ref() {
            return Ok(manager.clone());
        }
        drop(guard);

        let mut guard = REGISTRY
            .sdk_manager
            .write()
            .map_err(|_| RegistryAccessError::Poisoned)?;
        Ok(guard
            .get_or_insert_with(|| Arc::new(ShutdownManager::new_without_signals()))
            .clone())
    }

    /// Check if an SDK manager has been created.
    /// Useful for testing and debugging.
    #[allow(dead_code)]
    pub(crate) fn has_sdk_manager() -> Result<bool, RegistryAccessError> {
        Ok(REGISTRY
            .sdk_manager
            .read()
            .map_err(|_| RegistryAccessError::Poisoned)?
            .is_some())
    }

    /// Clear the SDK manager.
    /// This is primarily for testing to ensure isolation between tests.
    #[cfg(test)]
    pub(crate) async fn clear() -> Result<(), RegistryAccessError> {
        *REGISTRY
            .sdk_manager
            .write()
            .map_err(|_| RegistryAccessError::Poisoned)? = None;
        Ok(())
    }
}

/// Global instance of the runtime registry.
/// Uses LazyLock for on-demand initialization.
static REGISTRY: LazyLock<RuntimeRegistry> = LazyLock::new(|| RuntimeRegistry {
    sdk_manager: RwLock::new(None),
});

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_or_create_sdk() {
        // Clear any existing manager
        let _ = RuntimeRegistry::clear().await;

        assert!(!RuntimeRegistry::has_sdk_manager().unwrap());

        let manager1 = RuntimeRegistry::get_or_create_sdk().unwrap();
        assert!(RuntimeRegistry::has_sdk_manager().unwrap());

        let manager2 = RuntimeRegistry::get_or_create_sdk().unwrap();
        // Should return the same instance
        assert!(Arc::ptr_eq(&manager1, &manager2));

        let _ = RuntimeRegistry::clear().await;
        assert!(!RuntimeRegistry::has_sdk_manager().unwrap());
    }
}
