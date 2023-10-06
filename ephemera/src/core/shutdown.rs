use log::info;

use tokio::sync::broadcast;
use tokio::task::JoinHandle;

pub(crate) struct ShutdownManager {
    pub(crate) shutdown_tx: broadcast::Sender<()>,
    pub(crate) _shutdown_rcv: broadcast::Receiver<()>,
    handles: Vec<JoinHandle<anyhow::Result<()>>>,
}

pub(crate) struct Shutdown {
    pub(crate) shutdown_signal_rcv: broadcast::Receiver<()>,
}

#[derive(Clone)]
pub struct Handle {
    pub(crate) shutdown_started: bool,
}

impl Handle {
    /// Shutdown the node.
    /// This will send a shutdown signal to all tasks and wait for them to finish.
    ///
    /// # Errors
    /// This will return an error if shutdown signal can't be sent.
    ///
    /// # Panics
    /// This will panic if shutdown signal can't be sent.
    pub fn shutdown(&mut self) {
        self.shutdown_started = true;
    }
}

impl ShutdownManager {
    pub(crate) fn init() -> (ShutdownManager, Handle) {
        let (shutdown_tx, shutdown_rcv) = broadcast::channel(1);
        let shutdown_handle = Handle {
            shutdown_started: false,
        };
        let manager = Self {
            shutdown_tx,
            _shutdown_rcv: shutdown_rcv,
            handles: vec![],
        };
        (manager, shutdown_handle)
    }

    pub async fn stop(self) {
        info!("Starting Ephemera shutdown");
        self.shutdown_tx.send(()).unwrap();
        info!("Waiting for tasks to finish");
        for (i, handle) in self
            .handles
            .into_iter()
            .enumerate()
            .map(|(i, h)| (i + 1, h))
        {
            match handle.await.unwrap() {
                Ok(()) => info!("Task {i} finished successfully"),
                Err(e) => info!("Task {i} finished with error: {e}",),
            }
        }
        info!("All tasks finished");
    }

    pub(crate) fn subscribe(&self) -> Shutdown {
        let shutdown = self.shutdown_tx.subscribe();
        Shutdown {
            shutdown_signal_rcv: shutdown,
        }
    }

    pub(crate) fn add_handle(&mut self, handle: JoinHandle<anyhow::Result<()>>) {
        self.handles.push(handle);
    }
}
