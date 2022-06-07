// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use tokio::sync::watch::{self, error::SendError};

const SHUTDOWN_TIMER_SECS: u64 = 5;

/// Used to notify other tasks to gracefully shutdown
#[derive(Debug)]
pub struct ShutdownNotifier {
    notify_tx: watch::Sender<()>,
    notify_rx: Option<watch::Receiver<()>>,
}

impl Default for ShutdownNotifier {
    fn default() -> Self {
        let (notify_tx, notify_rx) = watch::channel(());
        Self {
            notify_tx,
            notify_rx: Some(notify_rx),
        }
    }
}

impl ShutdownNotifier {
    pub fn subscribe(&self) -> ShutdownListener {
        ShutdownListener::new(
            self.notify_rx
                .as_ref()
                .expect("Unable to subscribe to shutdown notifier that is already shutdown")
                .clone(),
        )
    }

    pub fn signal_shutdown(&self) -> Result<(), SendError<()>> {
        self.notify_tx.send(())
    }

    pub async fn wait_for_shutdown(&mut self) {
        if let Some(notify_rx) = self.notify_rx.take() {
            drop(notify_rx);
        }

        tokio::select! {
            _ = self.notify_tx.closed() => {
                log::info!("All registered tasks succesfully shutdown");
            },
            _ =  tokio::signal::ctrl_c() => {
                log::info!("Forcing shutdown");
            }
            _ = tokio::time::sleep(Duration::from_secs(SHUTDOWN_TIMER_SECS)) => {
                log::info!("Timout reached, forcing shutdown");
            },
        }
    }
}

/// Listen for shutdown notifications
#[derive(Clone, Debug)]
pub struct ShutdownListener {
    shutdown: bool,
    notify: watch::Receiver<()>,
}

impl ShutdownListener {
    pub fn new(notify: watch::Receiver<()>) -> ShutdownListener {
        ShutdownListener {
            shutdown: false,
            notify,
        }
    }

    pub fn empty() -> ShutdownListener {
        let (_, rx) = watch::channel(());
        ShutdownListener {
            shutdown: false,
            notify: rx,
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    pub async fn recv(&mut self) {
        if self.shutdown {
            return;
        }
        let _ = self.notify.changed().await;
        self.shutdown = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn signal_shutdown() {
        let shutdown = ShutdownNotifier::default();
        let mut listener = shutdown.subscribe();

        let task = tokio::spawn(async move {
            tokio::select! {
                _ = listener.recv() => 42,
            }
        });

        shutdown.signal_shutdown().unwrap();
        assert_eq!(task.await.unwrap(), 42);
    }
}
