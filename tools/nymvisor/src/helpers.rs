// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::future::Aborted;
use futures::stream::AbortHandle;
use std::future::Future;
use tokio::task::{JoinError, JoinHandle};

pub(crate) struct TaskHandle<T>
where
    T: Send + 'static,
{
    abort_handle: AbortHandle,
    // join_handle: JoinHandle<F::Output>,
    join_handle: JoinHandle<Result<T, Aborted>>,
}

impl<T> TaskHandle<T>
where
    T: Send + 'static,
{
    pub(crate) fn new(
        abort_handle: AbortHandle,
        join_handle: JoinHandle<Result<T, Aborted>>,
    ) -> Self {
        TaskHandle {
            abort_handle,
            join_handle,
        }
    }

    // TODO: change return type
    pub(crate) async fn abort_and_finalize(self) -> Result<Result<T, Aborted>, JoinError> {
        self.abort_handle.abort();
        self.join_handle.await
    }
}
