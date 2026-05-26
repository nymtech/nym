// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Tunnel lifecycle state. wasm32 builds with `panic = "abort"`, so
//! panic detection uses a chained hook + static atomic (see
//! [`install_panic_recorder`]) rather than `catch_unwind`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use nym_wasm_client_core::nym_task::ShutdownToken;

static WASM_PANICKED: AtomicBool = AtomicBool::new(false);

/// Chain a recorder onto the existing panic hook. Call once after
/// `nym_wasm_utils::set_panic_hook()`.
pub(crate) fn install_panic_recorder() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        WASM_PANICKED.store(true, Ordering::SeqCst);
        prev(info);
    }));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskName {
    Bridge,
    Reactor,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FailureReason {
    TaskExited { task: TaskName },
    TaskPanicked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum TunnelState {
    Connecting,
    Ready,
    ShuttingDown,
    Shutdown,
    Failed { reason: FailureReason },
}

#[derive(Clone)]
pub(crate) struct State {
    inner: Arc<Mutex<TunnelState>>,
    /// Cancelled by [`State::fail`]; should point at smolmix_tracker, not base.
    cascade: ShutdownToken,
}

impl State {
    pub(crate) fn new(cascade: ShutdownToken) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TunnelState::Connecting)),
            cascade,
        }
    }

    /// Reads the panic flag first; post-panic always returns Failed.
    pub(crate) fn get(&self) -> TunnelState {
        if WASM_PANICKED.load(Ordering::SeqCst) {
            return TunnelState::Failed {
                reason: FailureReason::TaskPanicked,
            };
        }
        self.inner.lock().unwrap().clone()
    }

    pub(crate) fn is_ready(&self) -> bool {
        matches!(self.get(), TunnelState::Ready)
    }

    pub(crate) fn set(&self, new: TunnelState) {
        *self.inner.lock().unwrap() = new;
    }

    /// Call at the end of each task body; no-op if the token was cancelled.
    pub(crate) fn finalise_task(&self, task: TaskName, token: &ShutdownToken) {
        if token.is_cancelled() {
            return;
        }
        self.fail(FailureReason::TaskExited { task });
    }

    /// First-failure-wins. Reads inner state directly (not via `get()`)
    /// so the panic short-circuit doesn't suppress the cascade.
    pub(crate) fn fail(&self, reason: FailureReason) {
        use TunnelState::*;
        {
            let mut state = self.inner.lock().unwrap();
            if matches!(*state, Shutdown | ShuttingDown | Failed { .. }) {
                return;
            }
            *state = Failed { reason };
        }
        self.cascade.cancel();
    }
}
