use crate::mixnet::{MixnetClient, MixnetClientBuilder, NymNetworkDetails};
use anyhow::{bail, Result};
use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

// Make a set # of clients (low default)
// Make sure that # of clients is always 1 above the # of incoming conn requests
// Once a client is used, kill the client & remove it from the pool
pub struct ClientPool {
    clients: Arc<RwLock<Vec<Arc<MixnetClient>>>>,
    default_pool_size: usize,
    conn_count: Arc<AtomicUsize>, // the actual # of connections running, denoting an incoming tcp request that is matched with a nym client
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
            .field("connection count", &*self.conn_count)
            .field("clients", &format_args!("[{}]", clients_debug));

        debug_struct.finish()
    }
}

impl ClientPool {
    pub fn new(default_pool_size: usize) -> Self {
        ClientPool {
            clients: Arc::new(RwLock::new(Vec::new())),
            default_pool_size,
            conn_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    // if clients == default, sleep
    // if incoming conns > default - conn_count (aka in use clients) then make more clients
    pub async fn start(&self) -> Result<()> {
        loop {
            let spawned_clients = self.clients.read().await.len();
            info!("Currently spawned clients: {}", spawned_clients);

            if spawned_clients >= self.default_pool_size {
                debug!("got enough clients already: sleeping");
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

    pub async fn get_mixnet_client(&self) -> Result<MixnetClient> {
        todo!()
    }

    // disconnect ephemeral
    // remove from vec
    pub async fn disconnect_and_remove_client(&self, client: MixnetClient) -> Result<()> {
        todo!()
    }

    pub async fn get_client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    pub fn get_conn_count(&self) -> usize {
        self.conn_count.load(Ordering::SeqCst)
    }

    pub fn increment_conn_count(&self) {
        self.conn_count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement_conn_count(&self) -> Result<()> {
        if self.get_conn_count() == 0 {
            bail!("count already 0");
        }
        self.conn_count.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn clone(&self) -> Self {
        Self {
            clients: Arc::clone(&self.clients),
            default_pool_size: *&self.default_pool_size,
            conn_count: Arc::clone(&self.conn_count),
        }
    }
}

// TODO COVER ALL FNS
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::thread;

    #[test]
    fn test_conn_count_increment_decrement() -> Result<()> {
        let tracker = ClientPool::new(0);
        tracker.increment_conn_count();
        tracker.increment_conn_count();
        assert_eq!(
            tracker.get_conn_count(),
            2,
            "should be 2 after single increment"
        );
        tracker.decrement_conn_count()?;
        assert_eq!(
            tracker.get_conn_count(),
            1,
            "should be 1 after two increments and one decrement"
        );
        Ok(())
    }
    #[test]
    fn test_clone() {
        let tracker = ClientPool::new(1);
        let tracker_clone = tracker.clone();

        tracker.increment_conn_count();
        assert_eq!(
            tracker_clone.get_conn_count(),
            1,
            "tracker clones should share the same count"
        );
    }

    #[test]
    fn test_conn_count_multiple_threads() {
        let tracker = ClientPool::new(0);
        let mut handles = vec![];

        for _ in 0..10 {
            let thread_tracker = tracker.clone();
            let handle = thread::spawn(move || {
                thread_tracker.increment_conn_count();
                thread::sleep(std::time::Duration::from_millis(10));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(
            tracker.get_conn_count(),
            10,
            "should be 10 after 10 thread increments"
        );
    }

    #[test]
    fn test_concurrent_increment_decrement() -> Result<()> {
        let tracker = ClientPool::new(0);
        let mut handles = vec![];

        for i in 0..10 {
            let thread_tracker = tracker.clone();
            let handle = thread::spawn(move || {
                if i < 5 {
                    thread_tracker.increment_conn_count();
                } else {
                    thread_tracker.decrement_conn_count().unwrap();
                }
                thread::sleep(std::time::Duration::from_millis(10));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(
            tracker.get_conn_count(),
            0,
            "should be 0 after equal increments and decrements"
        );
        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_zero_floor() {
        let tracker = ClientPool::new(0);
        tracker.decrement_conn_count().unwrap();
    }

    #[test]
    fn test_stress() {
        let tracker = ClientPool::new(0);
        let mut handles = vec![];
        let num_threads = 100;

        for _ in 0..num_threads {
            let thread_tracker = tracker.clone();
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    thread_tracker.increment_conn_count();
                    thread::sleep(std::time::Duration::from_micros(1));
                    thread_tracker.decrement_conn_count().unwrap();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(
            tracker.get_conn_count(),
            0,
            "should return to 0 after all increments and decrements"
        );
    }
}
