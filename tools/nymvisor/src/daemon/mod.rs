// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::{Config, DEFAULT_SHUTDOWN_GRACE_PERIOD};
use crate::error::NymvisorError;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::fs;
use std::future::Future;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::{sleep, Sleep};
use tracing::{debug, info, instrument, warn};

pub(crate) struct InterruptHandle(Arc<Notify>);

impl InterruptHandle {
    pub(crate) fn interrupt_daemon(&self) {
        self.0.notify_one()
    }
}

impl Drop for InterruptHandle {
    fn drop(&mut self) {
        self.interrupt_daemon();
    }
}

#[derive(Debug)]
pub(crate) struct Daemon {
    executable_path: PathBuf,
    kill_timeout: Duration,
}

impl Daemon {
    pub(crate) fn from_config(config: &Config) -> Self {
        Daemon {
            executable_path: config.current_daemon_binary(),
            kill_timeout: config.daemon.debug.shutdown_grace_period,
        }
    }

    pub(crate) fn new<P: AsRef<Path>>(executable_path: P) -> Self {
        Daemon {
            executable_path: executable_path.as_ref().to_path_buf(),
            kill_timeout: DEFAULT_SHUTDOWN_GRACE_PERIOD,
        }
    }

    #[instrument(skip(self), fields(self.executable_path = ?self.executable_path))]
    pub(crate) fn get_build_information(
        &self,
    ) -> Result<BinaryBuildInformationOwned, NymvisorError> {
        info!("attempting to obtain daemon build information");

        // TODO: do we need any timeouts here or could we just assume this is not going to take an eternity to execute?
        // I'm leaning towards the former
        let raw = std::process::Command::new(&self.executable_path)
            .args(["--no-banner", "build-info", "--output=json"])
            .output()
            .map_err(|source| NymvisorError::DaemonBuildInformationFailure {
                binary_path: self.executable_path.clone(),
                source,
            })?;

        debug!("execution status: {}", raw.status);

        if !raw.status.success() {
            return Err(raw.status.into());
        }

        serde_json::from_slice(&raw.stdout)
            .map_err(|source| NymvisorError::DaemonBuildInformationParseFailure { source })
    }

    #[instrument(skip(self), fields(self.executable_path = ?self.executable_path))]
    pub(crate) fn verify_binary(&self) -> Result<(), NymvisorError> {
        let metadata = fs::metadata(&self.executable_path).map_err(|source| {
            NymvisorError::MetadataReadFailure {
                path: self.executable_path.clone(),
                source,
            }
        })?;

        if !metadata.is_file() {
            return Err(NymvisorError::DaemonNotAFile {
                path: self.executable_path.clone(),
            });
        }

        let mut permissions = metadata.permissions();
        let mode = permissions.mode();
        let is_executable = mode & 0o111 != 0;
        if !is_executable {
            warn!(
                "the binary does not seem to have executable bits sets. attempting to fix that..."
            );
            let new_mode = mode | 0o111; // Set the three execute bits to on (a+x).
            permissions.set_mode(new_mode);

            fs::set_permissions(&self.executable_path, permissions).map_err(|source| {
                NymvisorError::DaemonPermissionFailure {
                    path: self.executable_path.clone(),
                    source,
                }
            })?;
        }

        Ok(())
    }

    pub(crate) fn execute_async<I, S>(&self, args: I) -> Result<ExecutingDaemon, NymvisorError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        // // TODO: we might have to pass env here
        let child = tokio::process::Command::new(&self.executable_path)
            .args(args)
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|source| NymvisorError::DaemonIoFailure { source })?;

        ExecutingDaemon::new(self.kill_timeout, child)
    }
}

#[must_use = "futures do nothing unless polled"]
#[non_exhaustive]
pub(crate) struct ExecutingDaemon {
    child_id: i32,
    kill_timeout_duration: Duration,
    interrupt_sent: bool,
    interrupt_handle: Option<Arc<Notify>>,

    // TODO: can we maybe get rid of that dynamic dispatch here in favour of concrete types?
    // interrupted: Option<Pin<Box<Notified<'static>>>>,
    interrupted: Pin<Box<dyn Future<Output = ()> + Send + Sync>>,
    kill_timeout: Option<Pin<Box<Sleep>>>,
    child_future: Pin<Box<dyn Future<Output = std::io::Result<ExitStatus>> + Send + Sync>>,
    // child_future: futures::future::BoxFuture<>
}

impl ExecutingDaemon {
    fn new(
        kill_timeout_duration: Duration,
        mut child: tokio::process::Child,
    ) -> Result<ExecutingDaemon, NymvisorError> {
        if let Some(id) = child.id() {
            let interrupt_handle = Arc::new(Notify::new());
            let notified_handle = Arc::clone(&interrupt_handle);
            Ok(ExecutingDaemon {
                child_id: id as i32,
                kill_timeout_duration,
                interrupt_sent: false,
                interrupt_handle: Some(interrupt_handle),
                interrupted: Box::pin(async move { notified_handle.notified().await }),
                kill_timeout: None,
                child_future: Box::pin(async move { child.wait().await }),
            })
        } else {
            // safety: if the child didn't return an id it means it has already terminated so it must be ready
            #[allow(clippy::expect_used)]
            Err(child
                .try_wait()
                .map_err(|source| NymvisorError::DaemonIoFailure { source })?
                .expect("finished child did not return an exit status")
                .into())
        }
    }

    pub(crate) fn interrupt_handle(&mut self) -> InterruptHandle {
        #[allow(clippy::expect_used)]
        InterruptHandle(
            self.interrupt_handle
                .take()
                .expect("the interrupt handle has already been obtained"),
        )
    }

    fn signal_child(&self, signal: Signal) -> Result<(), NymvisorError> {
        info!("sending {signal} to the daemon");
        nix::sys::signal::kill(Pid::from_raw(self.child_id), signal)
            .map_err(|source| NymvisorError::DaemonSignalFailure { signal, source })
    }
}

impl Future for ExecutingDaemon {
    type Output = Result<ExitStatus, NymvisorError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 1. check if the child is done
        if let Poll::Ready(result) = Pin::new(&mut self.child_future).poll(cx) {
            return match result {
                Ok(exit_status) => Poll::Ready(Ok(exit_status)),
                Err(source) => Poll::Ready(Err(NymvisorError::DaemonIoFailure { source })),
            };
        }

        // 2. check if we reached the timeout to kill the child
        if let Some(ref mut kill_timeout) = &mut self.kill_timeout {
            if kill_timeout.as_mut().poll(cx).is_ready() {
                warn!("reached the graceful shutdown timeout. the daemon still hasn't finished. sending SIGKILL");
                self.signal_child(Signal::SIGKILL)?;
                self.kill_timeout = None;
            }
        }

        // 3. check if we received a signal to interrupt the child
        // note: Notified is a fused future so there's no point in polling it after it already finished
        // safety: self.interrupted is always `Some` so the unwrap is fine
        #[allow(clippy::unwrap_used)]
        if !self.interrupt_sent && Pin::new(&mut self.interrupted).poll(cx).is_ready() {
            assert!(self.kill_timeout.is_none());

            self.signal_child(Signal::SIGINT)?;
            self.interrupt_sent = true;

            // it seems we have to poll the future here to make sure it's registered for waking the waker
            // note: this is guaranteed to either produce Poll::Ready or polling the kill timeout future
            cx.waker().wake_by_ref();
            self.kill_timeout = Some(Box::pin(sleep(self.kill_timeout_duration)));
        }

        Poll::Pending
    }
}
