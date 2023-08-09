// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::{error::Error, time::Duration};

use futures::{future::pending, FutureExt, SinkExt, StreamExt};
use tokio::{
    sync::{
        mpsc,
        watch::{self, error::SendError},
    },
    time::sleep,
};

const DEFAULT_SHUTDOWN_TIMER_SECS: u64 = 5;

pub(crate) type SentError = Box<dyn Error + Send + Sync>;
type ErrorSender = mpsc::UnboundedSender<SentError>;
type ErrorReceiver = mpsc::UnboundedReceiver<SentError>;

pub type SentStatus = Box<dyn Error + Send + Sync>;
pub type StatusSender = futures::channel::mpsc::Sender<SentStatus>;
pub type StatusReceiver = futures::channel::mpsc::Receiver<SentStatus>;

#[derive(thiserror::Error, Debug)]
enum TaskError {
    #[error("Task halted unexpectedly")]
    UnexpectedHalt,
}

// TODO: possibly we should create a `Status` trait instead of reusing `Error`
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    #[error("Ready")]
    Ready,
}

/// Listens to status and error messages from tasks, as well as notifying them to gracefully
/// shutdown. Keeps track of if task stop unexpectedly, such as in a panic.
#[derive(Debug)]
pub struct TaskManager {
    // These channels have the dual purpose of signalling it's time to shutdown, but also to keep
    // track of which tasks we are still waiting for.
    notify_tx: watch::Sender<()>,
    notify_rx: Option<watch::Receiver<()>>,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    shutdown_timer_secs: u64,

    // If any task failed, it needs to report separately
    task_return_error_tx: ErrorSender,
    task_return_error_rx: Option<ErrorReceiver>,

    // Also signal when the notifier is dropped, in case the task exits unexpectedly.
    // Why are we not reusing the return error channel? Well, let me tell you kids, it's because I
    // didn't manage to reliably get the explicitly sent error (and not the error sent during drop)
    task_drop_tx: ErrorSender,
    task_drop_rx: Option<ErrorReceiver>,

    // A task might also send non-fatal errors (effectively, warnings) while running that is not
    // the result of exiting.
    task_status_tx: StatusSender,
    task_status_rx: Option<StatusReceiver>,
}

impl Default for TaskManager {
    fn default() -> Self {
        let (notify_tx, notify_rx) = watch::channel(());
        let (task_halt_tx, task_halt_rx) = mpsc::unbounded_channel();
        let (task_drop_tx, task_drop_rx) = mpsc::unbounded_channel();
        // The status channel is bounded (unlike the others), since it's not always the case that
        // there is a listener.
        let (task_status_tx, task_status_rx) = futures::channel::mpsc::channel(128);
        Self {
            notify_tx,
            notify_rx: Some(notify_rx),
            shutdown_timer_secs: DEFAULT_SHUTDOWN_TIMER_SECS,
            task_return_error_tx: task_halt_tx,
            task_return_error_rx: Some(task_halt_rx),
            task_drop_tx,
            task_drop_rx: Some(task_drop_rx),
            task_status_tx,
            task_status_rx: Some(task_status_rx),
        }
    }
}

impl TaskManager {
    pub fn new(shutdown_timer_secs: u64) -> Self {
        Self {
            shutdown_timer_secs,
            ..Default::default()
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn catch_interrupt(mut self) -> Result<(), SentError> {
        let res = crate::wait_for_signal_and_error(&mut self).await;

        log::info!("Sending shutdown");
        self.signal_shutdown().ok();

        log::info!("Waiting for tasks to finish... (Press ctrl-c to force)");
        self.wait_for_shutdown().await;

        res
    }

    pub fn subscribe(&self) -> TaskClient {
        TaskClient::new(
            self.notify_rx
                .as_ref()
                .expect("Unable to subscribe to shutdown notifier that is already shutdown")
                .clone(),
            self.task_return_error_tx.clone(),
            self.task_drop_tx.clone(),
            self.task_status_tx.clone(),
        )
    }

    pub fn signal_shutdown(&self) -> Result<(), SendError<()>> {
        self.notify_tx.send(())
    }

    pub async fn start_status_listener(&mut self, mut sender: StatusSender) {
        // Announce that we are operational. This means that in the application where this is used,
        // everything is up and running and ready to go.
        if let Err(msg) = sender.send(Box::new(TaskStatus::Ready)).await {
            log::error!("Error sending status message: {}", msg);
        };

        if let Some(mut task_status_rx) = self.task_status_rx.take() {
            log::info!("Starting status message listener");
            crate::spawn::spawn(async move {
                loop {
                    if let Some(msg) = task_status_rx.next().await {
                        log::trace!("Got msg: {}", msg);
                        if let Err(msg) = sender.send(msg).await {
                            log::error!("Error sending status message: {}", msg);
                        }
                    } else {
                        log::trace!("Stopping since channel closed");
                        break;
                    }
                }
                log::debug!("Status listener: Exiting");
            });
        }
    }

    pub async fn wait_for_error(&mut self) -> Option<SentError> {
        let mut error_rx = self
            .task_return_error_rx
            .take()
            .expect("Unable to wait for error: attempt to wait twice?");
        let mut drop_rx = self
            .task_drop_rx
            .take()
            .expect("Unable to wait for error: attempt to wait twice?");

        // During an error we are likely like to be swamped with drop notifications as well, this
        // is a crude way to give priority to real errors (if there are any).
        let drop_rx = drop_rx.recv().then(|msg| async move {
            sleep(Duration::from_millis(50)).await;
            msg
        });

        tokio::select! {
            msg = error_rx.recv() => msg,
            msg = drop_rx => msg
        }
    }

    pub async fn wait_for_shutdown(&mut self) {
        log::debug!("Waiting for shutdown");
        if let Some(notify_rx) = self.notify_rx.take() {
            drop(notify_rx);
        }

        #[cfg(not(target_arch = "wasm32"))]
        let interrupt_future = tokio::signal::ctrl_c();

        // in wasm we'll never get our shutdown anyway...
        #[cfg(target_arch = "wasm32")]
        let interrupt_future = futures::future::pending::<()>();

        #[cfg(not(target_arch = "wasm32"))]
        let wait_future = tokio::time::sleep(Duration::from_secs(self.shutdown_timer_secs));

        // TODO: we should be using a `Delay` here for wasm
        #[cfg(target_arch = "wasm32")]
        let wait_future = futures::future::pending::<()>();

        tokio::select! {
            _ = self.notify_tx.closed() => {
                log::info!("All registered tasks succesfully shutdown");
            },
            _ = interrupt_future => {
                log::info!("Forcing shutdown");
            }
            _ = wait_future => {
                log::info!("Timeout reached, forcing shutdown");
            },
        }
    }
}

/// Listen for shutdown notifications, and can send error and status messages back to the
/// `TaskManager`
#[derive(Clone, Debug)]
pub struct TaskClient {
    // If a shutdown notification has been registered
    shutdown: bool,

    // Listen for shutdown notifications, as well as a mechanism to report back that we have
    // finished (the receiver is closed).
    notify: watch::Receiver<()>,

    // Send back error if we stopped
    return_error: ErrorSender,

    // Also notify if we dropped without shutdown being registered
    drop_error: ErrorSender,

    // Send non-exit messages
    status_msg: StatusSender,

    // The current operating mode
    mode: ClientOperatingMode,
}

impl TaskClient {
    #[cfg(not(target_arch = "wasm32"))]
    const SHUTDOWN_TIMEOUT_WAITING_FOR_SIGNAL_ON_EXIT: Duration = Duration::from_secs(5);

    fn new(
        notify: watch::Receiver<()>,
        return_error: ErrorSender,
        drop_error: ErrorSender,
        status_msg: StatusSender,
    ) -> TaskClient {
        TaskClient {
            shutdown: false,
            notify,
            return_error,
            drop_error,
            status_msg,
            mode: ClientOperatingMode::Listening,
        }
    }

    pub async fn run_future<Fut, T>(&mut self, fut: Fut) -> Option<T>
    where
        Fut: Future<Output = T>,
    {
        tokio::select! {
            biased;
            _ = self.recv() => None,
            res = fut => Some(res)
        }
    }

    // Create a dummy that will never report that we should shutdown.
    pub fn dummy() -> TaskClient {
        let (_notify_tx, notify_rx) = watch::channel(());
        let (task_halt_tx, _task_halt_rx) = mpsc::unbounded_channel();
        let (task_drop_tx, _task_drop_rx) = mpsc::unbounded_channel();
        let (task_status_tx, _task_status_rx) = futures::channel::mpsc::channel(128);
        TaskClient {
            shutdown: false,
            notify: notify_rx,
            return_error: task_halt_tx,
            drop_error: task_drop_tx,
            status_msg: task_status_tx,
            mode: ClientOperatingMode::Dummy,
        }
    }

    pub fn is_dummy(&self) -> bool {
        self.mode.is_dummy()
    }

    pub fn is_shutdown(&self) -> bool {
        if self.mode.is_dummy() {
            false
        } else {
            self.shutdown
        }
    }

    pub async fn recv(&mut self) {
        if self.mode.is_dummy() {
            return pending().await;
        }
        if self.shutdown {
            return;
        }
        let _ = self.notify.changed().await;
        self.shutdown = true;
    }

    pub async fn recv_with_delay(&mut self) {
        self.recv()
            .then(|msg| async move {
                sleep(Duration::from_secs(2)).await;
                msg
            })
            .await
    }

    pub async fn recv_timeout(&mut self) {
        if self.mode.is_dummy() {
            return pending().await;
        }
        #[cfg(not(target_arch = "wasm32"))]
        tokio::time::timeout(
            Self::SHUTDOWN_TIMEOUT_WAITING_FOR_SIGNAL_ON_EXIT,
            self.recv(),
        )
        .await
        .expect("Task stopped without shutdown called");
    }

    pub fn is_shutdown_poll(&mut self) -> bool {
        if self.mode.is_dummy() {
            return false;
        }
        if self.shutdown {
            return true;
        }
        match self.notify.has_changed() {
            Ok(has_changed) => {
                if has_changed {
                    self.shutdown = true;
                }
                has_changed
            }
            Err(err) => {
                log::error!("Polling shutdown failed: {err}");
                log::error!("Assuming this means we should shutdown...");
                true
            }
        }
    }

    // This listener should to *not* notify the ShutdownNotifier to shutdown when dropped. For
    // example when we clone the listener for a task handling connections, we often want to drop
    // without signal failure.
    pub fn mark_as_success(&mut self) {
        self.mode.set_should_not_signal_on_drop();
    }

    pub fn send_we_stopped(&mut self, err: SentError) {
        if self.mode.is_dummy() {
            return;
        }
        log::trace!("Notifying we stopped: {err}");
        if self.return_error.send(err).is_err() {
            log::error!("Failed to send back error message");
        }
    }

    pub fn send_status_msg(&mut self, msg: SentStatus) {
        if self.mode.is_dummy() {
            return;
        }
        // Since it's not always the case that anyone is listening, just try send and ignore any
        // failures.
        self.status_msg.try_send(msg).ok();
    }
}

impl Drop for TaskClient {
    fn drop(&mut self) {
        if !self.mode.should_signal_on_drop() {
            return;
        }
        if !self.is_shutdown_poll() {
            log::trace!("Notifying stop on unexpected drop");
            // If we can't send, well then there is not much to do
            self.drop_error
                .send(Box::new(TaskError::UnexpectedHalt))
                .ok();
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ClientOperatingMode {
    // Normal operations
    Listening,
    // Normal operations, but we don't report back if the we stop by getting dropped.
    ListeningButDontReportHalt,
    // Dummy mode, for when we don't do anything at all.
    Dummy,
}

impl ClientOperatingMode {
    fn is_dummy(&self) -> bool {
        self == &ClientOperatingMode::Dummy
    }

    fn should_signal_on_drop(&self) -> bool {
        match self {
            ClientOperatingMode::Listening => true,
            ClientOperatingMode::ListeningButDontReportHalt | ClientOperatingMode::Dummy => false,
        }
    }

    fn set_should_not_signal_on_drop(&mut self) {
        use ClientOperatingMode::{Dummy, Listening, ListeningButDontReportHalt};
        *self = match &self {
            ListeningButDontReportHalt | Listening => ListeningButDontReportHalt,
            Dummy => Dummy,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn signal_shutdown() {
        let shutdown = TaskManager::default();
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
