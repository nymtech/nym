//! Global registry for HTTP client configurations.
//!
//! This module provides a compile-time registry system that allows any crate
//! in the workspace to contribute configuration modifications to HTTP clients.

use crate::ReqwestClientBuilder;

/// A configuration record that modifies a `ReqwestClientBuilder`.
///
/// Records are collected at compile-time via the `inventory` crate and
/// applied in priority order when building HTTP clients.
pub struct ConfigRecord {
    /// Lower numbers run earlier.
    pub priority: i32,
    /// A function that takes a builder and returns a mutated builder.
    pub apply: fn(ReqwestClientBuilder) -> ReqwestClientBuilder,
}

inventory::collect!(ConfigRecord);

/// Returns the default builder with all registered configurations applied.
pub fn default_builder() -> ReqwestClientBuilder {
    let mut b = ReqwestClientBuilder::new();

    #[cfg(feature = "debug-inventory")]
    let mut test_client = ReqwestClientBuilder::new();

    let mut records: Vec<&'static ConfigRecord> =
        inventory::iter::<ConfigRecord>.into_iter().collect();
    records.sort_by_key(|r| r.priority); // lower runs first

    #[cfg(feature = "debug-inventory")]
    {
        eprintln!(
            "[HTTP-INVENTORY] Building client with {} registered configurations",
            records.len()
        );
    }

    for r in records {
        b = (r.apply)(b);
        #[cfg(feature = "debug-inventory")]
        {
            test_client = (r.apply)(test_client);
        }
    }

    #[cfg(feature = "debug-inventory")]
    {
        eprintln!("[HTTP-INVENTORY] Final builder state (Debug):");
        eprintln!("{:#?}", b);
        eprintln!(
            "[HTTP-INVENTORY] Note: reqwest::ClientBuilder doesn't expose all internal state"
        );
        eprintln!("[HTTP-INVENTORY] Building test client to verify configuration...");

        // Try to build a client to see if it works
        match test_client.build() {
            Ok(client) => {
                eprintln!("[HTTP-INVENTORY] ✓ Client built successfully");
                eprintln!("[HTTP-INVENTORY] Client debug info: {:#?}", client);
            }
            Err(e) => {
                eprintln!("[HTTP-INVENTORY] ✗ Failed to build client: {}", e);
            }
        }
    }

    b
}

/// Builds a client using the default builder with all registered configurations.
pub fn build_client() -> reqwest::Result<reqwest::Client> {
    default_builder().build()
}

/// Debug function to inspect registered configurations.
/// Returns a vector of (priority, function_pointer) tuples for debugging.
pub fn inspect_registered_configs() -> Vec<(i32, usize)> {
    let mut configs: Vec<(i32, usize)> = inventory::iter::<ConfigRecord>
        .into_iter()
        .map(|record| (record.priority, record.apply as usize))
        .collect();
    configs.sort_by_key(|(priority, _)| *priority);
    configs
}

/// Print all registered configurations to stderr for debugging.
/// This shows the priority and function pointer address of each registered config.
pub fn debug_print_inventory() {
    eprintln!("[HTTP-INVENTORY] Registered configurations:");
    let configs = inspect_registered_configs();
    if configs.is_empty() {
        eprintln!("  (none)");
    } else {
        for (i, (priority, ptr)) in configs.iter().enumerate() {
            eprintln!(
                "  [{:2}] Priority: {:4}, Function: 0x{:016x}",
                i, priority, ptr
            );
        }
        eprintln!("  Total: {} configurations", configs.len());
    }
}

/// Returns the count of registered configuration records.
pub fn registered_config_count() -> usize {
    inventory::iter::<ConfigRecord>.into_iter().count()
}
