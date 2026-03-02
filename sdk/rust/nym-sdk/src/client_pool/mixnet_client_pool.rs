// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet::{MixnetClient, MixnetClientBuilder, NymNetworkDetails};
use anyhow::Result;
use nym_crypto::asymmetric::ed25519;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

/// A pool of connected ephemeral [`MixnetClient`] instances for higher-throughput applications.
///
/// `ClientPool` maintains a configurable number of ready-to-use Mixnet clients in reserve,
/// automatically creating new clients when the pool is depleted. This is useful for
/// applications that need to handle many concurrent connections without the latency
/// of creating new clients on-demand.
///
/// ## Usage
///
/// The pool operates as a background task that continuously maintains the configured
/// number of clients. Clients are obtained via [`get_mixnet_client`](Self::get_mixnet_client)
/// and are removed from the pool (then disconnected).
///
/// ## Example
///
/// ```rust,no_run
/// use nym_sdk::client_pool::ClientPool;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     // Create a pool that maintains 5 clients in reserve
///     let pool = ClientPool::new(5);
///
///     // Start the pool in a background task
///     let pool_clone = pool.clone();
///     tokio::spawn(async move {
///         pool_clone.start().await
///     });
///
///     // Get a client from the pool when needed
///     if let Some(client) = pool.get_mixnet_client().await {
///         println!("Got client: {}", client.nym_address());
///         // Use the client...
///         client.disconnect().await;
///     }
///
///     // Shutdown the pool
///     pool.disconnect_pool().await;
///     Ok(())
/// }
/// ```
pub struct ClientPool {
    /// Collection of clients waiting to be used which are popped off in get_mixnet_client()
    clients: Arc<RwLock<Vec<Arc<MixnetClient>>>>,
    /// Default # of clients to have available in pool in reserve waiting for incoming connections
    client_pool_reserve_number: usize,
    /// CancellationToken used to signal shutdown
    cancel_token: CancellationToken,
}

// This is only necessary for when you're wanting to check the addresses of the clients that are currently in the pool via logging.
impl fmt::Debug for ClientPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let clients_debug = match self.clients.try_read() {
            Ok(clients) => {
                if f.alternate() {
                    // pretty
                    clients
                        .iter()
                        .enumerate()
                        .map(|(i, client)| format!("\n      {}: {}", i, client.nym_address()))
                        .collect::<Vec<_>>()
                        .join(",")
                } else {
                    // compact
                    clients
                        .iter()
                        .map(|client| client.nym_address().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            }
            Err(_) => "<locked>".to_string(),
        };

        let mut debug_struct = f.debug_struct("Pool");
        debug_struct
            .field(
                "client_pool_reserve_number",
                &self.client_pool_reserve_number,
            )
            .field("clients", &format_args!("[{clients_debug}]"));
        debug_struct.finish()
    }
}

impl Clone for ClientPool {
    fn clone(&self) -> Self {
        Self {
            clients: Arc::clone(&self.clients),
            client_pool_reserve_number: self.client_pool_reserve_number,
            cancel_token: self.cancel_token.clone(),
        }
    }
}

impl ClientPool {
    /// Creates a new client pool with the specified reserve size.
    ///
    /// The pool will attempt to maintain `client_pool_reserve_number` clients
    /// ready for immediate use. The pool starts empty and must be activated
    /// by calling [`start`](Self::start).
    ///
    /// # Arguments
    ///
    /// * `client_pool_reserve_number` - The target number of clients to keep in reserve.
    ///   Set to 0 to create a pool that doesn't automatically spawn clients.
    pub fn new(client_pool_reserve_number: usize) -> Self {
        ClientPool {
            clients: Arc::new(RwLock::new(Vec::new())),
            client_pool_reserve_number,
            cancel_token: CancellationToken::new(),
        }
    }

    /// Starts the pool's background task that maintains the client reserve.
    ///
    /// This method runs a loop that continuously checks if more clients are needed
    /// and creates them as necessary. The loop continues until [`disconnect_pool`](Self::disconnect_pool)
    /// is called.
    ///
    /// This should typically be spawned as a background task:
    ///
    /// ```rust,no_run
    /// # use nym_sdk::client_pool::ClientPool;
    /// # async fn example() {
    /// let pool = ClientPool::new(3);
    /// let pool_clone = pool.clone();
    /// tokio::spawn(async move {
    ///     let _ = pool_clone.start().await;
    /// });
    /// # }
    /// ```
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the pool is shut down via cancellation token.
    pub async fn start(&self) -> Result<()> {
        loop {
            let spawned_clients = self.clients.read().await.len();
            let addresses = self;
            debug!(
                "Currently spawned clients: {}: {:?}",
                spawned_clients, addresses
            );
            if self.cancel_token.is_cancelled() {
                break Ok(());
            }
            if spawned_clients >= self.client_pool_reserve_number {
                debug!("Got enough clients already: sleeping");
            } else {
                info!(
                    "Clients in reserve = {}, reserve amount = {}, spawning new client",
                    spawned_clients, self.client_pool_reserve_number
                );
                let client = loop {
                    let net = NymNetworkDetails::new_from_env();
                    match MixnetClientBuilder::new_ephemeral()
                        .network_details(net)
                        .build()?
                        .connect_to_mixnet()
                        .await
                    {
                        Ok(client) => break client,
                        Err(err) => {
                            warn!("Error creating client: {:?}, will retry in 100ms", err);
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                    }
                };
                self.clients.write().await.push(Arc::new(client));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    /// Starts the pool with all clients connecting to a specific gateway.
    ///
    /// This variant of [`start`](Self::start) forces all created clients to use the
    /// specified gateway. Primarily useful for testing scenarios where gateway
    /// consistency is required.
    ///
    /// # Arguments
    ///
    /// * `gateway` - The Ed25519 public key of the gateway all clients should connect to.
    pub async fn start_with_specified_gateway(&self, gateway: ed25519::PublicKey) -> Result<()> {
        loop {
            let spawned_clients = self.clients.read().await.len();
            let addresses = self;
            debug!(
                "Currently spawned clients: {}: {:?}",
                spawned_clients, addresses
            );
            if self.cancel_token.is_cancelled() {
                break Ok(());
            }
            if spawned_clients >= self.client_pool_reserve_number {
                debug!("Got enough clients already: sleeping");
            } else {
                info!(
                    "Clients in reserve = {}, reserve amount = {}, spawning new client",
                    spawned_clients, self.client_pool_reserve_number
                );
                let client = loop {
                    let net = NymNetworkDetails::new_from_env();
                    match MixnetClientBuilder::new_ephemeral()
                        .network_details(net)
                        .request_gateway(gateway.to_string())
                        .build()?
                        .connect_to_mixnet()
                        .await
                    {
                        Ok(client) => break client,
                        Err(err) => {
                            warn!("Error creating client: {:?}, will retry in 100ms", err);
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                    }
                };
                self.clients.write().await.push(Arc::new(client));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    /// Shuts down the pool and disconnects all clients.
    ///
    /// This method:
    /// 1. Cancels the background task that creates new clients
    /// 2. Disconnects all clients currently in the pool
    ///
    /// After calling this method, the pool cannot be restarted. Create a new
    /// `ClientPool` instance if you need to resume pooling.
    pub async fn disconnect_pool(&self) {
        info!("Triggering Client Pool disconnect");
        self.cancel_token.cancel();
        info!(
            "Client pool cancellation token cancelled: {}",
            self.cancel_token.is_cancelled()
        );
        let mut clients = self.clients.write().await;
        while let Some(arc_client) = clients.pop() {
            if let Ok(client) = Arc::try_unwrap(arc_client) {
                info!("Killing reserve client {}", client.nym_address());
                client.disconnect().await;
            }
        }
    }

    /// Retrieves a client from the pool, if one is available.
    ///
    /// The client is removed from the pool and ownership is transferred to the caller.
    /// After use, the client should be disconnected; it is not returned to the pool.
    ///
    /// If the pool is empty, this returns `None`. The background task started by
    /// [`start`](Self::start) will create a replacement client automatically.
    ///
    /// # Returns
    ///
    /// - `Some(MixnetClient)` if a client was available in the pool
    /// - `None` if the pool is currently empty
    pub async fn get_mixnet_client(&self) -> Option<MixnetClient> {
        debug!("Grabbing client from pool");
        let mut clients = self.clients.write().await;
        clients
            .pop()
            .and_then(|arc_client| Arc::try_unwrap(arc_client).ok())
    }

    /// Returns the current number of clients available in the pool.
    pub async fn get_client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Returns the configured reserve size for this pool.
    ///
    /// This is the target number of clients the pool attempts to maintain,
    /// as set during construction with [`new`](Self::new).
    pub async fn get_pool_reserve(&self) -> usize {
        self.client_pool_reserve_number
    }
}
