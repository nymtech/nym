// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::CredentialProxyError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex as StdMutex, MutexGuard};
use tokio::task::JoinHandle;
use tracing::{debug, error};

pub(super) type RefillTaskResult = Result<(), CredentialProxyError>;

#[derive(Default)]
pub(super) struct RefillTask {
    // note that we can only have a single transaction in progress (or it'd mess up with our sequence numbers)
    // if we find that we're using up deposits more quickly than we're refilling them,
    // we'll have to increase the number of deposits per transaction
    join_handle: StdMutex<Option<JoinHandle<RefillTaskResult>>>,

    in_progress: AtomicBool,
}

impl RefillTask {
    /// Attempt to set the `in_progress` value to `true` if it's not already `true`.
    /// Returns boolean indicating whether it was successful
    fn try_set_in_progress(&self) -> bool {
        self.in_progress
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub(super) fn try_get_new_task_guard(
        &self,
    ) -> Option<MutexGuard<'_, Option<JoinHandle<RefillTaskResult>>>> {
        // sanity check for concurrent request
        if !self.try_set_in_progress() {
            debug!("another task has already started deposit refill request");
            return None;
        }

        #[allow(clippy::expect_used)]
        let guard = self.join_handle.lock().expect("mutex got poisoned");

        if let Some(existing_handle) = guard.as_ref() {
            if !existing_handle.is_finished() {
                error!("CRITICAL BUG: there was already a deposit refill task spawned that hasn't yet finished")
            }
        }

        Some(guard)
    }

    pub(super) fn take_task_join_handle(&self) -> Option<JoinHandle<RefillTaskResult>> {
        #[allow(clippy::expect_used)]
        self.join_handle.lock().expect("mutex got poisoned").take()
    }
}
