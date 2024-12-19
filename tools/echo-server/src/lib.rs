// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Result;
use nym_crypto::asymmetric::ed25519;
use nym_sdk::mixnet::Recipient;
use nym_sdk::tcp_proxy;
use nym_sdk::tcp_proxy::NymProxyServer;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::task;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

pub struct Metrics {
    total_conn: AtomicU64,
    active_conn: AtomicU64,
    bytes_recv: AtomicU64,
    bytes_sent: AtomicU64,
}

impl Metrics {
    fn new() -> Self {
        Self {
            total_conn: AtomicU64::new(0),
            active_conn: AtomicU64::new(0),
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
}

impl NymEchoServer {
    pub async fn new(
        gateway: Option<ed25519::PublicKey>,
        config_path: Option<&str>,
        env: String,
        listen_port: &str,
    ) -> Result<Self> {
        let home_dir = dirs::home_dir().expect("Unable to get home directory");
        let default_path = format!("{}/tmp/nym-proxy-server-config", home_dir.display());
        let config_path = config_path.unwrap_or(&default_path);
        let listen_addr = format!("127.0.0.1:{}", listen_port);

        Ok(NymEchoServer {
            client: Arc::new(Mutex::new(
                tcp_proxy::NymProxyServer::new(
                    &listen_addr,
                    &config_path,
                    Some(env.clone()),
                    gateway,
                )
                .await?,
            )),
            listen_addr,
            metrics: Arc::new(Metrics::new()),
            cancel_token: CancellationToken::new(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let cancel_token = self.cancel_token.clone();

        let client = Arc::clone(&self.client);
        task::spawn(async move {
            client.lock().await.run_with_shutdown().await?;
            Ok::<(), anyhow::Error>(())
        });

        let all_metrics = Arc::clone(&self.metrics);

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                info!(
                    "Metrics: total_connections={}, active_connections={}, bytes_received={}, bytes_sent={}",
                    all_metrics.total_conn.load(Ordering::Relaxed),
                    all_metrics.active_conn.load(Ordering::Relaxed),
                    all_metrics.bytes_recv.load(Ordering::Relaxed),
                    all_metrics.bytes_sent.load(Ordering::Relaxed),
                );
            }
        });

        let listener = TcpListener::bind(self.listen_addr.clone()).await?;
        info!("{listener:?}");

        loop {
            tokio::select! {
                stream = listener.accept() => {
                    let (stream, _) = stream?;
                    info!("Handling new stream");
                    let connection_metrics = Arc::clone(&self.metrics);
                    connection_metrics.total_conn.fetch_add(1, Ordering::Relaxed);
                    connection_metrics.active_conn.fetch_add(1, Ordering::Relaxed);

                    tokio::spawn(NymEchoServer::handle_incoming(
                        stream, connection_metrics, cancel_token.clone()
                    ));
                }
                _ = self.cancel_token.cancelled() => {
                    info!("Cancel token cancelled: {:?}", self.cancel_token.cancelled());
                    break Ok(());
                }
            }
        }
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
                    warn!("Shutdown signal received, closing connection");
                    break;
                }
            }
        }
        metrics
            .active_conn
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        info!("Connection closed");
    }

    pub async fn disconnect(&self) {
        self.cancel_token.cancel();
        let client = Arc::clone(&self.client);
        client.lock().await.disconnect().await;
        while self.metrics.active_conn.load(Ordering::Relaxed) > 0 {
            info!("Waiting on active connections to close: sleeping");
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    pub async fn nym_address(&self) -> Recipient {
        self.client.lock().await.nym_address().clone()
    }

    pub fn listen_addr(&self) -> String {
        self.listen_addr.clone()
    }
}
