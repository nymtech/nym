use crate::mixnet::{MixnetClient, MixnetClientBuilder, NymNetworkDetails};
use anyhow::Result;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

// Make a set # of clients (low default)
// Once a client is used, kill the client & remove it from the pool
pub struct ClientPool {
    clients: Arc<RwLock<Vec<Arc<MixnetClient>>>>,
    semaphore: Arc<Semaphore>,
    default_pool_size: usize, // default # of clients to have available in pool. If incoming tcp requests > this, make ephemeral client elsewhere
}

impl fmt::Debug for ClientPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let clients_debug = match self.clients.try_read() {
            Ok(clients) => {
                if f.alternate() {
                    // pretty
                    clients
                        .iter()
                        .enumerate()
                        .map(|(i, client)| {
                            format!("\n      {}: {}", i, client.nym_address().to_string())
                        })
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
            .field("default_pool_size", &self.default_pool_size)
            .field("clients", &format_args!("[{}]", clients_debug));
        debug_struct.finish()
    }
}

impl ClientPool {
    pub fn new(default_pool_size: usize) -> Self {
        ClientPool {
            clients: Arc::new(RwLock::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(default_pool_size)),
            default_pool_size,
        }
    }

    pub async fn start(&self) -> Result<()> {
        loop {
            let spawned_clients = self.clients.read().await.len();
            let addresses = self;
            info!(
                "Currently spawned clients: {}: {:?} ",
                spawned_clients, addresses
            );
            // TODO PROBLEM IS HERE: not updating / tracking the in use permits when grab_mixnet_client is called
            info!(
                "current avail permits {}",
                self.semaphore.available_permits()
            );
            info!(
                "current in use permits {}",
                self.default_pool_size - self.semaphore.available_permits()
            );
            if spawned_clients == self.semaphore.available_permits() {
                debug!("Got enough clients already: sleeping");
            } else {
                info!("Spawning new client");
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
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    pub async fn get_mixnet_client(&self) -> Option<MixnetClient> {
        info!("Grabbing client from pool");
        let permit = self.semaphore.acquire().await;
        info!("{permit:?}");
        info!("Available permits: {}", self.semaphore.available_permits());
        let mut clients = self.clients.write().await;
        // gain ownership of client, tracking with semaphore once its working to stop constantly renewing size of pool to default_pool_size and instead have pool be (default_pool_size - in use clients) to stop bloat
        clients
            .pop()
            .and_then(|arc_client| Arc::try_unwrap(arc_client).ok())
    }

    // This might still be needed if it needs to be called with a cancellation token in various threads. keeping for the moment
    // pub async fn disconnect_and_remove_client(&self, client: MixnetClient) -> Result<()> {
    //     client.disconnect().await;
    //     Ok(())
    // }

    pub async fn get_client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    pub fn clone(&self) -> Self {
        Self {
            clients: Arc::clone(&self.clients),
            semaphore: Arc::clone(&self.semaphore),
            default_pool_size: *&self.default_pool_size,
        }
    }
}
