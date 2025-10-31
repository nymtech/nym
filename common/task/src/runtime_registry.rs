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

    #[error("The SDK ShutdownManager already exists")]
    ExistingShutdownManager,

    #[error("No existing SDK ShutdownManager")]
    MissingShutdownManager,
}

impl RuntimeRegistry {
    /// Create a ShutdownManager for SDK use.
    /// This manager doesn't listen to OS signals, making it suitable for library use.
    /// This function overwrite any existing manager!
    pub(crate) fn create_sdk() -> Result<Arc<ShutdownManager>, RegistryAccessError> {
        let mut guard = REGISTRY
            .sdk_manager
            .write()
            .map_err(|_| RegistryAccessError::Poisoned)?;

        Ok(guard
            .insert(Arc::new(
                ShutdownManager::new_without_signals().with_cancel_on_panic(),
            ))
            .clone())
    }

    /// Get the  ShutdownManager for SDK use.
    /// This manager doesn't listen to OS signals, making it suitable for library use.
    /// Not yet used, but maybe in the future
    #[allow(dead_code)]
    pub(crate) fn get_sdk() -> Result<Arc<ShutdownManager>, RegistryAccessError> {
        let guard = REGISTRY
            .sdk_manager
            .read()
            .map_err(|_| RegistryAccessError::Poisoned)?;
        if let Some(manager) = guard.as_ref() {
            Ok(manager.clone())
        } else {
            Err(RegistryAccessError::MissingShutdownManager)
        }
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

        // Error if nothing was created
        assert!(RuntimeRegistry::get_sdk().is_err());

        let manager1 = RuntimeRegistry::create_sdk().unwrap();
        assert!(RuntimeRegistry::has_sdk_manager().unwrap());

        let manager2 = RuntimeRegistry::get_sdk().unwrap();
        // Should return the same instance
        assert!(Arc::ptr_eq(&manager1, &manager2));

        let _ = RuntimeRegistry::clear().await;
        assert!(!RuntimeRegistry::has_sdk_manager().unwrap());
    }
}
