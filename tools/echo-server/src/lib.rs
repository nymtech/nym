// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Result;
use nym_crypto::asymmetric::ed25519;
use nym_sdk::mixnet::{MixnetClient, MixnetClientSender, Recipient};
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

#[derive(Clone)]
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
                    "Metrics: total_connections_since_start={},  bytes_received={}, bytes_sent={}",
                    all_metrics.total_conn.load(Ordering::Relaxed),
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

        info!("Connection closed");
    }

    pub async fn disconnect(&self) {
        self.cancel_token.cancel();
        let client = Arc::clone(&self.client);
        client.lock().await.disconnect().await;
    }

    pub async fn nym_address(&self) -> Recipient {
        self.client.lock().await.nym_address().clone()
    }

    pub fn listen_addr(&self) -> String {
        self.listen_addr.clone()
    }

    pub fn metrics(&self) -> Arc<Metrics> {
        self.metrics.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use nym_sdk::mixnet::{MixnetClient, MixnetMessageSender, Recipient};

    #[tokio::test]
    async fn echoes_bytes() {
        let mut echo_server =
            NymEchoServer::new(None, None, "../../envs/mainnet.env".to_string(), "9000")
                .await
                .unwrap();

        let echo_addr = echo_server.nym_address().await;
        println!("{echo_addr}");
        let incoming_metrics = echo_server.clone().metrics();
        println!("{incoming_metrics:#?}");

        tokio::task::spawn(async move {
            echo_server.run().await.unwrap();
        });

        let message = "test";

        let mut client = MixnetClient::connect_new().await.unwrap();
        let sender = client.split_sender();
        tokio::spawn(async move {
            sender.send_plain_message(echo_addr, message).await.unwrap();
        });

        tokio::spawn(async move {
            if let Some(received) = client.next().await {
                println!("Received: {}", String::from_utf8_lossy(&received.message));
                assert_eq!(
                    message.to_string(),
                    String::from_utf8_lossy(&received.message)
                );
            }
            client.disconnect().await;
        });

        // tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        // assert_eq!(
        //     incoming_metrics.bytes_recv.load(Ordering::SeqCst) as usize,
        //     message_bytes.len()
        // );
    }

    // #[test]
    // fn creates_a_valid_nym_addr_with_given_gw() {
    //     // check valid
    //     // parse end
    // }
}
