// Copyright (c) 2023 Nym Technologies S.A., GPL-3.0
//
// Based on mini-redis, Copyright (c) 2020 Tokio Contributors, MIT
// Copyright (c) 2020 Tokio Contributors
use std::{future::Future, pin::Pin};

use tokio::sync::broadcast;

#[derive(Debug)]
pub struct Shutdown {
    /// `true` if the shutdown signal has been received
    shutdown: bool,

    /// The receive half of the channel used to listen for shutdown.
    notify: broadcast::Receiver<()>,
}

impl Shutdown {
    /// Create a new `Shutdown` backed by the given `broadcast::Receiver`.
    pub fn new(notify: broadcast::Receiver<()>) -> Shutdown {
        Shutdown {
            shutdown: false,
            notify,
        }
    }

    /// Returns `true` if the shutdown signal has been received.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    /// Receive the shutdown notice, waiting if necessary.
    pub async fn recv(&mut self) {
        // If the shutdown signal has already been received, then return
        // immediately.
        if self.shutdown {
            return;
        }

        // Cannot receive a "lag error" as only one value is ever sent.
        let _ = self.notify.recv().await;

        // Remember that the signal has been received.
        self.shutdown = true;
    }
}

pub struct ShutdownManager {
    shutdown_notifier: tokio::sync::broadcast::Sender<()>,
}

impl ShutdownManager {
    pub fn new() -> Self {
        let (shutdown_notifier, _) = tokio::sync::broadcast::channel(1);
        Self { shutdown_notifier }
    }

    pub fn shutdown_received_future(&self) -> Pin<Box<impl Future<Output = ()>>> {
        let mut shutdown = self.new_shutdown();
        Box::pin(async move { shutdown.recv().await })
    }

    pub fn new_shutdown(&self) -> Shutdown {
        Shutdown::new(self.shutdown_notifier.subscribe())
    }

    // OS signal is broadcasted to rest of the system through broadcaster
    pub async fn register_signal_handler(self) {
        tracing::info!("registering signal handler");
        tokio::spawn(async move {
            let ctrl_c = async {
                tokio::signal::ctrl_c()
                    .await
                    .expect("failed to install Ctrl+C handler");
            };

            #[cfg(unix)]
            let terminate = async {
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("failed to install TERM signal handler")
                    .recv()
                    .await;
            };

            #[cfg(not(unix))]
            let terminate = std::future::pending::<()>();

            tokio::select! {
                _ = ctrl_c => {},
                _ = terminate => {},
            }

            let _ = self.shutdown_notifier.send(());
            tracing::info!("Shutdown signal received. Starting graceful shutdown");
        });
    }

    // OS signal is broadcasted to rest of the system through broadcaster
    #[cfg(windows)]
    pub async fn register_signal_handler_windows(self, shutdown_rx: std::sync::mpsc::Receiver<()>) {
        tracing::info!("registering signal handler");
        tokio::spawn(async move {
            let ctrl_c = async {
                tokio::signal::ctrl_c()
                    .await
                    .expect("failed to install Ctrl+C handler");
            };

            let terminate = async move {
                let _ = shutdown_rx.recv();
            };

            tokio::select! {
                _ = ctrl_c => {},
                _ = terminate => {},
            }

            let _ = self.shutdown_notifier.send(());
            tracing::info!("Shutdown signal received. Starting graceful shutdown");
        });
    }
}
