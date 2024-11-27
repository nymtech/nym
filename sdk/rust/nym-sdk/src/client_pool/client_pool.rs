use crate::mixnet::{MixnetClient, MixnetClientBuilder, NymNetworkDetails};
use anyhow::Result;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub struct ClientPool {
    clients: Arc<RwLock<Vec<Arc<MixnetClient>>>>, // collection of clients waiting to be used which are popped off in get_mixnet_client()
    client_pool_reserve_number: usize, // default # of clients to have available in pool in reserve waiting for incoming connections
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
            .field(
                "client_pool_reserve_number",
                &self.client_pool_reserve_number,
            )
            .field("clients", &format_args!("[{}]", clients_debug));
        debug_struct.finish()
    }
}

impl ClientPool {
    pub fn new(client_pool_reserve_number: usize) -> Self {
        ClientPool {
            clients: Arc::new(RwLock::new(Vec::new())),
            client_pool_reserve_number,
        }
    }

    pub async fn start(&self) -> Result<()> {
        loop {
            let spawned_clients = self.clients.read().await.len();
            let addresses = self;
            debug!(
                "Currently spawned clients: {}: {:?} ",
                spawned_clients, addresses
            );
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

    pub async fn get_mixnet_client(&self) -> Option<MixnetClient> {
        debug!("Grabbing client from pool");
        let mut clients = self.clients.write().await;
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
            client_pool_reserve_number: *&self.client_pool_reserve_number,
        }
    }
}
