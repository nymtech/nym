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
    let mut records: Vec<&'static ConfigRecord> =
        inventory::iter::<ConfigRecord>.into_iter().collect();
    records.sort_by_key(|r| r.priority); // lower runs first
    for r in records {
        b = (r.apply)(b);
    }
    b
}

/// Builds a client using the default builder with all registered configurations.
pub fn build_client() -> reqwest::Result<reqwest::Client> {
    default_builder().build()
}

/// Debug function to inspect registered configurations.
/// Returns a vector of (priority, function_pointer) tuples for debugging.
#[cfg(debug_assertions)]
pub fn inspect_registered_configs() -> Vec<(i32, usize)> {
    let mut configs: Vec<(i32, usize)> = inventory::iter::<ConfigRecord>
        .into_iter()
        .map(|record| (record.priority, record.apply as usize))
        .collect();
    configs.sort_by_key(|(priority, _)| *priority);
    configs
}

/// Returns the count of registered configuration records.
pub fn registered_config_count() -> usize {
    inventory::iter::<ConfigRecord>.into_iter().count()
}
