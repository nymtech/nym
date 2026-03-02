//! A variable-sized pool of ephemeral Mixnet clients for higher-throughput applications.
//!
//! This module provides [`ClientPool`], which maintains a configurable number of
//! connected ephemeral [`MixnetClient`](crate::mixnet::MixnetClient) instances. This is
//! useful for applications that need to handle many concurrent connections without
//! the latency of creating new clients on-demand.
//!
//! # Example
//!
//! ```rust,no_run
//! use nym_sdk::client_pool::ClientPool;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a pool that maintains 5 clients in reserve
//!     let pool = ClientPool::new(5);
//!
//!     // Start the pool in a background task
//!     let pool_clone = pool.clone();
//!     tokio::spawn(async move {
//!         pool_clone.start().await
//!     });
//!
//!     // Get a client from the pool when needed
//!     if let Some(client) = pool.get_mixnet_client().await {
//!         println!("Got client: {}", client.nym_address());
//!         client.disconnect().await;
//!     }
//!
//!     // Shutdown the pool
//!     pool.disconnect_pool().await;
//!     Ok(())
//! }
//! ```

mod mixnet_client_pool;

pub use mixnet_client_pool::ClientPool;
