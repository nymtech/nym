// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Result;
use nym_crypto::asymmetric::ed25519;
use nym_sdk::{mixnet::Recipient, tcp_proxy, tcp_proxy::NymProxyServer};
use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::{broadcast, Mutex},
    task,
};
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

const METRICS_TICK: u8 = 6; // Tempo of metrics logging in seconds

#[derive(Debug)]
pub struct Metrics {
    total_conn: AtomicU64,
    bytes_recv: AtomicU64,
    bytes_sent: AtomicU64,
}

impl Metrics {
    fn new() -> Self {
        Self {
            total_conn: AtomicU64::new(0),
            bytes_recv: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
        }
    }
}

pub struct NymEchoServer {
    client: Arc<Mutex<NymProxyServer>>,
    listen_addr: String,
    metrics: Arc<Metrics>,
    cancel_token: CancellationToken,
    client_shutdown_tx: tokio::sync::mpsc::Sender<()>, // Shutdown signal for the TcpProxyServer instance
    shutdown_tx: tokio::sync::mpsc::Sender<()>,        // Shutdown signals for the EchoServer
    shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    ready_tx: broadcast::Sender<()>, // Signal for upstream code if consuming the crate showing readiness
}

impl NymEchoServer {
    pub async fn new(
        gateway: Option<ed25519::PublicKey>,
        config_path: Option<&str>,
        env: Option<String>,
        listen_port: &str,
    ) -> Result<Self> {
        let home_dir = dirs::home_dir().expect("Unable to get home directory");
        let default_path = format!("{}/tmp/nym-proxy-server-config", home_dir.display());
        let config_path = config_path.unwrap_or(&default_path);
        let listen_addr = format!("127.0.0.1:{listen_port}");

        let client = Arc::new(Mutex::new(
            tcp_proxy::NymProxyServer::new(&listen_addr, config_path, env, gateway).await?,
        ));

        let client_shutdown_tx = client.lock().await.disconnect_signal();

        let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);

        let (ready_tx, _) = broadcast::channel(1);

        Ok(NymEchoServer {
            client,
            listen_addr,
            metrics: Arc::new(Metrics::new()),
            cancel_token: CancellationToken::new(),
            client_shutdown_tx,
            shutdown_tx,
            shutdown_rx,
            ready_tx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let cancel_token = self.cancel_token.clone();

        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(METRICS_TICK as u64));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let client = Arc::clone(&self.client);
        task::spawn(async move {
            client.lock().await.run_with_shutdown().await?;
            Ok::<(), anyhow::Error>(())
        });

        let all_metrics = Arc::clone(&self.metrics);

        let listener = TcpListener::bind(self.listen_addr.clone()).await?;
        debug!("{listener:?}");

        let mut shutdown_rx =
            std::mem::replace(&mut self.shutdown_rx, tokio::sync::mpsc::channel(1).1);

        info!("Ready to accept incoming traffic");
        let _ = self.ready_tx.send(());

        loop {
            tokio::select! {
                Some(()) = shutdown_rx.recv() => {
                    info!("Disconnect signal received");
                    self.cancel_token.cancel();
                    info!("Cancel token cancelled: killing handle_incoming loops");
                     self.client_shutdown_tx.send(()).await?;
                     info!("Sent shutdown signal to ProxyServer instance");
                    break;
                }
                stream = listener.accept() => {
                    let (stream, _) = stream?;
                    info!("Handling new stream");
                    let connection_metrics = Arc::clone(&self.metrics);
                    connection_metrics.total_conn.fetch_add(1, Ordering::Relaxed);

                    tokio::spawn(NymEchoServer::handle_incoming(
                        stream, connection_metrics, cancel_token.clone()
                    ));
                }
                _ = interval.tick() => {
                    info!("Metrics: total_connections_since_start={}, bytes_received={}, bytes_sent={}",
                        all_metrics.total_conn.load(Ordering::Relaxed),
                        all_metrics.bytes_recv.load(Ordering::Relaxed),
                        all_metrics.bytes_sent.load(Ordering::Relaxed),
                    );
                }
            }
        }
        self.shutdown_rx = shutdown_rx;
        Ok(())
    }

    async fn handle_incoming(
        socket: TcpStream,
        metrics: Arc<Metrics>,
        cancel_token: CancellationToken,
    ) {
        let (read, mut write) = socket.into_split();
        let codec = tokio_util::codec::BytesCodec::new();
        let mut framed_read = tokio_util::codec::FramedRead::new(read, codec);

        loop {
            tokio::select! {
                Some(result) = framed_read.next() => {
                    match result {
                        Ok(bytes) => {
                            let len = bytes.len();
                            metrics.bytes_recv.fetch_add(len as u64, Ordering::Relaxed);
                            if let Err(e) = write.write_all(&bytes).await {
                                error!("Failed to write to stream with err: {}", e);
                                break;
                            }
                            metrics.bytes_sent.fetch_add(len as u64, Ordering::Relaxed);
                        }
                        Err(e) => {
                            error!("Failed to read from stream with err: {}", e);
                            break;
                        }
                    }
                }
                _ = cancel_token.cancelled() => {
                    info!("Shutdown signal received, closing connection");
                    break;
                }
            }
        }

        info!("Connection closed");
    }

    pub fn disconnect_signal(&self) -> tokio::sync::mpsc::Sender<()> {
        self.shutdown_tx.clone()
    }

    pub async fn nym_address(&self) -> Recipient {
        *self.client.lock().await.nym_address()
    }

    pub fn listen_addr(&self) -> String {
        self.listen_addr.clone()
    }

    pub fn metrics(&self) -> Arc<Metrics> {
        self.metrics.clone()
    }

    pub fn ready_signal(&self) -> broadcast::Receiver<()> {
        self.ready_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use nym_sdk::mixnet::{IncludedSurbs, MixnetClient, MixnetMessageSender};
    use nym_sdk::tcp_proxy::{Payload, ProxiedMessage};
    use tempfile::TempDir;

    #[tokio::test]
    #[ignore]
    async fn shutdown_works() -> Result<()> {
        let config_dir = TempDir::new()?;
        let mut echo_server = match NymEchoServer::new(
            None,
            Some(config_dir.path().to_str().unwrap()),
            None, // Mainnet by default
            "9000",
        )
        .await
        {
            Ok(server) => server,
            Err(err) => {
                error!("{err}");
                // this is not an ideal way of checking it, but if test fails due to networking failures
                // it should be fine to progress
                if err.to_string().contains("nym api request failed") {
                    return Ok(());
                }
                return Err(err);
            }
        };

        // Getter for shutdown signal
        let shutdown_tx = echo_server.disconnect_signal();

        // Getter for ready signal
        let mut ready_rx = echo_server.ready_signal();

        // Start the echo serv
        let server_handle = tokio::spawn(async move { echo_server.run().await.unwrap() });

        // Wait until you can match on ready signal - you will see "Ready to accept incoming traffic" in echo server logs when running it as CLI
        loop {
            match ready_rx.try_recv() {
                Ok(()) => {
                    println!("Server is ready!");
                    break;
                }
                Err(broadcast::error::TryRecvError::Empty) => {
                    // Channel is still empty, wait a bit and try again
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Err(broadcast::error::TryRecvError::Closed) => {
                    return Err(anyhow::anyhow!(
                        "Ready channel closed before server was ready"
                    ));
                }
                Err(broadcast::error::TryRecvError::Lagged(_)) => {
                    // Broadcast channel was set before we checked but handle it anyway; server is ready
                    break;
                }
            }
        }

        // Kill server
        shutdown_tx.send(()).await?;

        // Wait for shutdown in handle
        server_handle.await?;

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn echoes_bytes() -> Result<()> {
        let config_dir = TempDir::new()?;
        let mut echo_server = match NymEchoServer::new(
            None,
            Some(config_dir.path().to_str().unwrap()),
            None,
            "9001",
        )
        .await
        {
            Ok(server) => server,
            Err(err) => {
                error!("{err}");
                // this is not an ideal way of checking it, but if test fails due to networking failures
                // it should be fine to progress
                if err.to_string().contains("nym api request failed") {
                    return Ok(());
                }
                return Err(err);
            }
        };

        let echo_addr = echo_server.nym_address().await;

        let shutdown_tx = echo_server.disconnect_signal();
        let mut ready_rx = echo_server.ready_signal();

        let server_handle = tokio::task::spawn(async move {
            echo_server.run().await.unwrap();
        });

        loop {
            match ready_rx.try_recv() {
                Ok(()) => {
                    println!("Server is ready!");
                    break;
                }
                Err(broadcast::error::TryRecvError::Empty) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Err(broadcast::error::TryRecvError::Closed) => {
                    return Err(anyhow::anyhow!(
                        "Ready channel closed before server was ready"
                    ));
                }
                Err(broadcast::error::TryRecvError::Lagged(_)) => {
                    break;
                }
            }
        }
        println!("Sending message");

        let session_id = uuid::Uuid::new_v4();
        let message_id = 0;
        let outgoing = ProxiedMessage::new(
            Payload::Data("test".as_bytes().to_vec()),
            session_id,
            message_id,
        );
        let coded_message = bincode::serialize(&outgoing)?;

        println!("sending {coded_message:?}");

        let mut client = MixnetClient::connect_new().await?;

        println!("sending client addr {}", client.nym_address());
        let sender = client.split_sender();

        let receiving_task_handle = tokio::spawn(async move {
            println!("in handle");
            if let Some(received) = client.next().await {
                println!("{received:?}");
                let incoming: ProxiedMessage = bincode::deserialize(&received.message).unwrap();
                assert_eq!(outgoing.message, incoming.message);
            }
            println!("disconnecting client");
            client.disconnect().await;
            println!("client disconnected");
        });

        println!("after recv task handle");

        let sending_task_handle = tokio::spawn(async move {
            sender
                .send_message(echo_addr, &coded_message, IncludedSurbs::Amount(10))
                .await
                .unwrap();
        });

        println!("after sending task handle");

        receiving_task_handle.await?;
        sending_task_handle.await?;

        println!("after handles resolve");

        shutdown_tx.send(()).await?;

        println!("sent shutdown");

        server_handle.await?;

        Ok(())
    }
}
