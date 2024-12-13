//! use crate::mixnet::{MixnetClient, MixnetClientBuilder, NymNetworkDetails};
//! use anyhow::Result;
//! use std::fmt;
//! use std::sync::Arc;
//! use tokio::sync::RwLock;
//! use tokio_util::sync::CancellationToken;
//! use tracing::{debug, info, warn};

//! pub struct ClientPool {
//!     clients: Arc<RwLock<Vec<Arc<MixnetClient>>>>, // Collection of clients waiting to be used which are popped off in get_mixnet_client()
//!     client_pool_reserve_number: usize, // Default # of clients to have available in pool in reserve waiting for incoming connections
//!     cancel_token: CancellationToken,
//! }

//! // This is only necessary for when you're wanting to check the addresses of the clients that are currently in the //! pool.
//! impl fmt::Debug for ClientPool {
//!     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//!         let clients_debug = match self.clients.try_read() {
//!             Ok(clients) => {
//!                 if f.alternate() {
//!                     // pretty
//!                     clients
//!                         .iter()
//!                         .enumerate()
//!                         .map(|(i, client)| format!("\n      {}: {}", i, client.nym_address()))
//!                         .collect::<Vec<_>>()
//!                         .join(",")
//!                 } else {
//!                     // compact
//!                     clients
//!                         .iter()
//!                         .map(|client| client.nym_address().to_string())
//!                         .collect::<Vec<_>>()
//!                         .join(", ")
//!                 }
//!             }
//!             Err(_) => "<locked>".to_string(),
//!         };

//!         let mut debug_struct = f.debug_struct("Pool");
//!         debug_struct
//!             .field(
//!                 "client_pool_reserve_number",
//!                 &self.client_pool_reserve_number,
//!             )
//!             .field("clients", &format_args!("[{}]", clients_debug));
//!         debug_struct.finish()
//!     }
//! }

//! impl ClientPool {
//!     pub fn new(client_pool_reserve_number: usize) -> Self {
//!         ClientPool {
//!             clients: Arc::new(RwLock::new(Vec::new())),
//!             client_pool_reserve_number,
//!             cancel_token: CancellationToken::new(),
//!         }
//!     }

//!     // The loop here is simple: if there aren't enough clients, create more. If you set clients to 0, repeatedly //! just sleep.
//!     // disconnect_pool() will kill this loop via the cancellation token.
//!     pub async fn start(&self) -> Result<()> {
//!         loop {
//!             let spawned_clients = self.clients.read().await.len();
//!             let addresses = self;
//!             debug!(
//!                 "Currently spawned clients: {}: {:?}",
//!                 spawned_clients, addresses
//!             );
//!             if self.cancel_token.is_cancelled() {
//!                 break Ok(());
//!             }
//!             if spawned_clients >= self.client_pool_reserve_number {
//!                 debug!("Got enough clients already: sleeping");
//!             } else {
//!                 info!(
//!                     "Clients in reserve = {}, reserve amount = {}, spawning new client",
//!                     spawned_clients, self.client_pool_reserve_number
//!                 );
//!                 let client = loop {
//!                     let net = NymNetworkDetails::new_from_env();
//!                     match MixnetClientBuilder::new_ephemeral()
//!                         .network_details(net)
//!                         .build()?
//!                         .connect_to_mixnet()
//!                         .await
//!                     {
//!                         Ok(client) => break client,
//!                         Err(err) => {
//!                             warn!("Error creating client: {:?}, will retry in 100ms", err);
//!                             tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
//!                         }
//!                     }
//!                 };
//!                 self.clients.write().await.push(Arc::new(client));
//!             }
//!             tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
//!         }
//!     }

//!     pub async fn disconnect_pool(&self) {
//!         info!("Triggering Client Pool disconnect");
//!         self.cancel_token.cancel();
//!         info!(
//!             "Client pool cancellation token cancelled: {}",
//!             self.cancel_token.is_cancelled()
//!         );
//!         let mut clients = self.clients.write().await;
//!         while let Some(arc_client) = clients.pop() {
//!             if let Ok(client) = Arc::try_unwrap(arc_client) {
//!                 info!("Killing reserve client {}", client.nym_address());
//!                 client.disconnect().await;
//!             }
//!         }
//!     }

//!     pub async fn get_mixnet_client(&self) -> Option<MixnetClient> {
//!         debug!("Grabbing client from pool");
//!         let mut clients = self.clients.write().await;
//!         clients
//!             .pop()
//!             .and_then(|arc_client| Arc::try_unwrap(arc_client).ok())
//!     }

//!     pub async fn get_client_count(&self) -> usize {
//!         self.clients.read().await.len()
//!     }

//!     pub async fn get_pool_reserve(&self) -> usize {
//!         self.client_pool_reserve_number
//!     }

//!     pub fn clone(&self) -> Self {
//!         Self {
//!             clients: Arc::clone(&self.clients),
//!             client_pool_reserve_number: self.client_pool_reserve_number,
//!             cancel_token: self.cancel_token.clone(),
//!         }
//!     }
//! }

mod client_pool;

pub use client_pool::ClientPool;
