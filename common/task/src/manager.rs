// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    error::Error,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use futures::{future::pending, FutureExt, SinkExt, StreamExt};
use log::{log, Level};
use tokio::sync::{
    mpsc,
    watch::{self, error::SendError},
};

use crate::event::{SentStatus, StatusReceiver, StatusSender, TaskStatus};

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::{sleep, timeout};

#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::{sleep, timeout};

const DEFAULT_SHUTDOWN_TIMER_SECS: u64 = 5;

pub(crate) type SentError = Box<dyn Error + Send + Sync>;
type ErrorSender = mpsc::UnboundedSender<SentError>;
type ErrorReceiver = mpsc::UnboundedReceiver<SentError>;

fn try_recover_name(name: &Option<String>) -> String {
    if let Some(name) = name {
        name.clone()
    } else {
        "unknown".to_string()
    }
}

#[derive(thiserror::Error, Debug)]
enum TaskError {
    #[error("Task '{}' halted unexpectedly", try_recover_name(.shutdown_name))]
    UnexpectedHalt { shutdown_name: Option<String> },
}

/// Listens to status and error messages from tasks, as well as notifying them to gracefully
/// shutdown. Keeps track of if task stop unexpectedly, such as in a panic.
#[derive(Debug)]
pub struct TaskManager {
    // optional name assigned to the task manager that all subscribed task clients will inherit
    name: Option<String>,

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
            name: None,
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

    #[must_use]
    pub fn named<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn catch_interrupt(&mut self) -> Result<(), SentError> {
        let res = crate::wait_for_signal_and_error(self).await;

        log::info!("Sending shutdown");
        self.signal_shutdown().ok();

        log::info!("Waiting for tasks to finish... (Press ctrl-c to force)");
        self.wait_for_shutdown().await;

        res
    }

    pub fn subscribe(&self) -> TaskClient {
        let task_client = TaskClient::new(
            self.notify_rx
                .as_ref()
                .expect("Unable to subscribe to shutdown notifier that is already shutdown")
                .clone(),
            self.task_return_error_tx.clone(),
            self.task_drop_tx.clone(),
            self.task_status_tx.clone(),
        );

        if let Some(name) = &self.name {
            task_client.named(format!("{name}-child"))
        } else {
            task_client
        }
    }

    pub fn subscribe_named<S: Into<String>>(&self, suffix: S) -> TaskClient {
        let task_client = self.subscribe();
        let suffix = suffix.into();
        let child_name = if let Some(base) = &self.name {
            format!("{base}-{suffix}")
        } else {
            format!("unknown-{suffix}")
        };
        task_client.named(child_name)
    }

    pub fn signal_shutdown(&self) -> Result<(), SendError<()>> {
        self.notify_tx.send(())
    }

    pub async fn start_status_listener(
        &mut self,
        mut sender: StatusSender,
        start_status: TaskStatus,
    ) {
        // Announce that we are operational. This means that in the application where this is used,
        // everything is up and running and ready to go.
        if let Err(msg) = sender.send(Box::new(start_status)).await {
            log::error!("Error sending status message: {}", msg);
        };

        if let Some(mut task_status_rx) = self.task_status_rx.take() {
            log::info!("Starting status message listener");
            crate::spawn::spawn(async move {
                loop {
                    if let Some(msg) = task_status_rx.next().await {
                        log::trace!("Got msg: {msg}");
                        if let Err(msg) = sender.send(msg).await {
                            log::error!("Error sending status message: {msg}");
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

        #[cfg(target_arch = "wasm32")]
        let interrupt_future = futures::future::pending::<()>();

        let wait_future = sleep(Duration::from_secs(self.shutdown_timer_secs));

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
#[derive(Debug)]
pub struct TaskClient {
    // optional name assigned to the shutdown handle
    name: Option<String>,

    // If a shutdown notification has been registered
    // the reason for having an atomic here is to be able to cheat and modify that value whilst
    // holding an immutable reference to the `TaskClient`.
    // note: using `Relaxed` ordering everywhere is fine since it's not shared between threads
    shutdown: AtomicBool,

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

impl Clone for TaskClient {
    fn clone(&self) -> Self {
        // make sure to not accidentally overflow the stack if we keep cloning the handle
        let name = if let Some(name) = &self.name {
            if name != Self::OVERFLOW_NAME && name.len() < Self::MAX_NAME_LENGTH {
                Some(format!("{name}-child"))
            } else {
                Some(Self::OVERFLOW_NAME.to_string())
            }
        } else {
            None
        };

        TaskClient {
            name,
            shutdown: AtomicBool::new(self.shutdown.load(Ordering::Relaxed)),
            notify: self.notify.clone(),
            return_error: self.return_error.clone(),
            drop_error: self.drop_error.clone(),
            status_msg: self.status_msg.clone(),
            mode: self.mode.clone(),
        }
    }
}

impl TaskClient {
    const MAX_NAME_LENGTH: usize = 128;
    const OVERFLOW_NAME: &'static str = "reached maximum TaskClient children name depth";

    const SHUTDOWN_TIMEOUT_WAITING_FOR_SIGNAL_ON_EXIT: Duration = Duration::from_secs(5);

    fn new(
        notify: watch::Receiver<()>,
        return_error: ErrorSender,
        drop_error: ErrorSender,
        status_msg: StatusSender,
    ) -> TaskClient {
        TaskClient {
            name: None,
            shutdown: AtomicBool::new(false),
            notify,
            return_error,
            drop_error,
            status_msg,
            mode: ClientOperatingMode::Listening,
        }
    }

    // TODO: not convinced about the name...
    pub fn fork<S: Into<String>>(&self, child_suffix: S) -> Self {
        let mut child = self.clone();
        let suffix = child_suffix.into();
        let child_name = if let Some(base) = &self.name {
            format!("{base}-{suffix}")
        } else {
            format!("unknown-{suffix}")
        };

        child.name = Some(child_name);
        child
    }

    // just a convenience wrapper for including the shutdown name when logging
    // I really didn't want to create macros for that... because that seemed like an overkill.
    // but I guess it would have resolved needing to call `format!` for additional msg arguments
    fn log<S: Into<String>>(&self, level: Level, msg: S) {
        let msg = msg.into();

        let target = &if let Some(name) = &self.name {
            format!("TaskClient-{name}")
        } else {
            "unnamed-TaskClient".to_string()
        };

        log!(target: target, level, "{}", format!("[{target}] {msg}"))
    }

    #[must_use]
    pub fn named<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    #[must_use]
    pub fn with_suffix<S: Into<String>>(self, suffix: S) -> Self {
        let suffix = suffix.into();
        let name = if let Some(base) = &self.name {
            format!("{base}-{suffix}")
        } else {
            format!("unknown-{suffix}")
        };
        self.named(name)
    }

    // Create a dummy that will never report that we should shutdown.
    pub fn dummy() -> TaskClient {
        let (_notify_tx, notify_rx) = watch::channel(());
        let (task_halt_tx, _task_halt_rx) = mpsc::unbounded_channel();
        let (task_drop_tx, _task_drop_rx) = mpsc::unbounded_channel();
        let (task_status_tx, _task_status_rx) = futures::channel::mpsc::channel(128);
        TaskClient {
            name: None,
            shutdown: AtomicBool::new(false),
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
            self.shutdown.load(Ordering::Relaxed)
        }
    }

    pub async fn recv(&mut self) {
        if self.mode.is_dummy() {
            return pending().await;
        }
        if self.shutdown.load(Ordering::Relaxed) {
            return;
        }
        let _ = self.notify.changed().await;
        self.shutdown.store(true, Ordering::Relaxed);
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

        if let Err(timeout) = timeout(
            Self::SHUTDOWN_TIMEOUT_WAITING_FOR_SIGNAL_ON_EXIT,
            self.recv(),
        )
        .await
        {
            self.log(Level::Error, "Task stopped without shutdown called");
            panic!("{:?}: {timeout}", self.name)
        }
    }

    pub fn is_shutdown_poll(&self) -> bool {
        if self.mode.is_dummy() {
            return false;
        }
        if self.shutdown.load(Ordering::Relaxed) {
            return true;
        }
        match self.notify.has_changed() {
            Ok(has_changed) => {
                if has_changed {
                    self.shutdown.store(true, Ordering::Relaxed);
                }
                has_changed
            }
            Err(err) => {
                self.log(Level::Error, format!("Polling shutdown failed: {err}"));
                self.log(Level::Error, "Assuming this means we should shutdown...");

                true
            }
        }
    }

    // This listener should to *not* notify the ShutdownNotifier to shutdown when dropped. For
    // example when we clone the listener for a task handling connections, we often want to drop
    // without signal failure.
    pub fn disarm(&mut self) {
        self.mode.set_should_not_signal_on_drop();
    }

    pub fn send_we_stopped(&mut self, err: SentError) {
        if self.mode.is_dummy() {
            return;
        }

        self.log(Level::Trace, format!("Notifying we stopped: {err}"));

        if self.return_error.send(err).is_err() {
            self.log(Level::Error, "failed to send back error message");
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
            self.log(
                Level::Trace,
                "the task client is getting dropped but inststructed to not signal: this is expected during client shutdown",
            );
            return;
        } else {
            self.log(
                Level::Debug,
                "the task client is getting dropped: this is expected during client shutdown",
            );
        }

        if !self.is_shutdown_poll() {
            self.log(Level::Trace, "Notifying stop on unexpected drop");

            // If we can't send, well then there is not much to do
            self.drop_error
                .send(Box::new(TaskError::UnexpectedHalt {
                    shutdown_name: self.name.clone(),
                }))
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

#[derive(Debug)]
pub enum TaskHandle {
    /// Full [`TaskManager`] that was created by the underlying task.
    Internal(TaskManager),

    /// `[TaskClient]` that was passed from an external task, that controls the shutdown process.
    External(TaskClient),
}

impl From<TaskManager> for TaskHandle {
    fn from(value: TaskManager) -> Self {
        TaskHandle::Internal(value)
    }
}

impl From<TaskClient> for TaskHandle {
    fn from(value: TaskClient) -> Self {
        TaskHandle::External(value)
    }
}

impl Default for TaskHandle {
    fn default() -> Self {
        TaskHandle::Internal(TaskManager::default())
    }
}

impl TaskHandle {
    #[must_use]
    pub fn name_if_unnamed<S: Into<String>>(self, name: S) -> Self {
        match self {
            TaskHandle::Internal(task_manager) => {
                if task_manager.name.is_none() {
                    TaskHandle::Internal(task_manager.named(name))
                } else {
                    TaskHandle::Internal(task_manager)
                }
            }
            TaskHandle::External(task_client) => {
                if task_client.name.is_none() {
                    TaskHandle::External(task_client.named(name))
                } else {
                    TaskHandle::External(task_client)
                }
            }
        }
    }

    #[must_use]
    pub fn named<S: Into<String>>(self, name: S) -> Self {
        match self {
            TaskHandle::Internal(task_manager) => TaskHandle::Internal(task_manager.named(name)),
            TaskHandle::External(task_client) => TaskHandle::External(task_client.named(name)),
        }
    }

    pub fn fork<S: Into<String>>(&self, child_suffix: S) -> TaskClient {
        match self {
            TaskHandle::External(shutdown) => shutdown.fork(child_suffix),
            TaskHandle::Internal(shutdown) => shutdown.subscribe_named(child_suffix),
        }
    }

    pub fn get_handle(&self) -> TaskClient {
        match self {
            TaskHandle::External(shutdown) => shutdown.clone(),
            TaskHandle::Internal(shutdown) => shutdown.subscribe(),
        }
    }

    pub fn try_into_task_manager(self) -> Option<TaskManager> {
        match self {
            TaskHandle::External(_) => None,
            TaskHandle::Internal(shutdown) => Some(shutdown),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn wait_for_shutdown(self) -> Result<(), SentError> {
        match self {
            TaskHandle::Internal(mut task_manager) => task_manager.catch_interrupt().await,
            TaskHandle::External(mut task_client) => {
                task_client.recv().await;
                Ok(())
            }
        }
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
