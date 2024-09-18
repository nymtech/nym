use anyhow::Result;
use bytes::Bytes;
use nym_sdk::tcp_proxy;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tokio::sync::broadcast;
use tokio::task;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use tracing_subscriber;

const HOST: &str = "127.0.0.1";
const PORT: &str = "9000";

struct Metrics {
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

#[tokio::main]
async fn main() -> Result<()> {
    // if you run this with DEBUG you see the msg buffer on the ProxyServer, but its quite chatty
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Configure our client to use the Canary test network: you can switch this to use any of the files in `../../../envs/`
    let env_path = "../../envs/canary.env".to_string();
    let conf_path = "./tmp/nym-proxy-server-config";
    let mut proxy_server = tcp_proxy::NymProxyServer::new(
        &(format!("{}:{}", HOST, PORT)),
        conf_path,
        Some(env_path.clone()),
    )
    .await
    .unwrap();
    let proxy_nym_addr = proxy_server.nym_address().clone();
    info!("ProxyServer listening out on {}", proxy_nym_addr);

    task::spawn(async move {
        let _ = proxy_server
            .run_with_shutdown()
            .await
            .expect("NymProxyServer shutdown unexpectedly");
    });

    let (shutdown_sender, _) = broadcast::channel(1);
    let metrics = Arc::new(Metrics::new());
    let all_metrics = Arc::clone(&metrics);

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

    let listener = TcpListener::bind(format!("{}:{}", HOST, PORT)).await?;

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Shutdown signal received, closing server...");
                let _ = shutdown_sender.send(());
                // TODO we need something like this for the ProxyServer client
                break;
            }
            Ok((socket, _)) = listener.accept() => {
                let connection_metrics = Arc::clone(&metrics);
                let shutdown_rx = shutdown_sender.subscribe();
                connection_metrics.total_conn.fetch_add(1, Ordering::Relaxed);
                connection_metrics.active_conn.fetch_add(1, Ordering::Relaxed);
                tokio::spawn(async move {
                    handle_incoming(socket, connection_metrics, shutdown_rx).await;
                });
            }
        }
    }

    signal::ctrl_c().await?;
    info!("Received CTRL+C");

    while metrics.active_conn.load(Ordering::Relaxed) > 0 {
        info!("Waiting on active connections to close: sleeping 100ms");
        // TODO some kind of hard kill here for the ProxyServer
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    Ok(())
}

async fn handle_incoming(
    socket: TcpStream,
    metrics: Arc<Metrics>,
    mut shutdown_rx: broadcast::Receiver<()>,
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
                        metrics.bytes_recv.fetch_add(len as u64, std::sync::atomic::Ordering::Relaxed);
                        if let Err(e) = write.write_all(&bytes).await {
                            error!("Failed to write to stream with err: {}", e);
                            break;
                        }
                        metrics.bytes_sent.fetch_add(len as u64, std::sync::atomic::Ordering::Relaxed);
                    }
                    Err(e) => {
                        error!("Failed to read from stream with err: {}", e);
                        break;
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                warn!("Shutdown signal received, closing connection");
                break;
            }
            // TODO need to work out a way that if this timesout and breaks but you dont hang up the conn on the client end you can reconnect..maybe. If we just use this as a ping echo server I dont think this is a problem
            // EDIT I'm not actually sure we want this functionality? Measuring active connections might be useful though
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(120)) => {
                info!("Timeout reached, assuming we wont get more messages on this conn, closing");
                let close_message = "Closing conn, reconnect if you want to ping again";
                let bytes: Bytes = close_message.into();
                write.write_all(&bytes).await.expect("Couldn't write to socket");
                break;
            }
        }
    }
    metrics
        .active_conn
        .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    info!("Connection closed");
}
