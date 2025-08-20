// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::future::BoxFuture;
use futures::{ready, FutureExt};
use std::mem;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Default)]
pub(crate) struct InnerWrapper<T: 'static> {
    pub(crate) buffer: Arc<Mutex<ContentWrapper<T>>>,
    lock_state: LockState<T>,
}

impl<T: Send> InnerWrapper<T> {
    pub(crate) fn clone_buffer(&self) -> Arc<Mutex<ContentWrapper<T>>> {
        Arc::clone(&self.buffer)
    }

    pub(crate) fn cloned_buffer(&self) -> Self {
        assert!(matches!(self.lock_state, LockState::Idle));
        InnerWrapper {
            buffer: self.clone_buffer(),
            lock_state: LockState::Idle,
        }
    }

    // NOTE: it's responsibility of the caller to ensure the guard is released and state transitions to idle!
    pub(crate) fn poll_guard_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        match &mut self.lock_state {
            LockState::Idle => {
                // 1. first try to obtain the guard without locking
                let Ok(guard) = self.buffer.clone().try_lock_owned() else {
                    // 2. if that fails, create the future for obtaining it
                    self.lock_state =
                        LockState::TryingToLock(self.buffer.clone().lock_owned().boxed());
                    return Poll::Pending;
                };

                // correctly transition to locked state and poll ourselves again
                self.lock_state = LockState::Locked(guard);
                cx.waker().wake_by_ref();
                Poll::Ready(())
            }

            LockState::TryingToLock(lock_fut) => {
                // see if the guard future has resolved, if so, transition to locked state and schedule for another poll
                let guard = ready!(lock_fut.as_mut().poll(cx));
                self.lock_state = LockState::Locked(guard);
                cx.waker().wake_by_ref();
                Poll::Pending
            }

            LockState::Locked(_) => Poll::Ready(()),
        }
    }

    pub(crate) fn guard(&mut self) -> Option<&mut OwnedMutexGuard<ContentWrapper<T>>> {
        match &mut self.lock_state {
            LockState::Locked(guard) => Some(guard),
            _ => None,
        }
    }

    pub(crate) fn transition_to_idle(&mut self) {
        self.lock_state = LockState::Idle
    }
}

#[derive(Default)]
pub(crate) enum LockState<T> {
    // We haven’t started locking yet
    #[default]
    Idle,

    // Waiting for the mutex lock future to resolve
    TryingToLock(BoxFuture<'static, OwnedMutexGuard<ContentWrapper<T>>>),

    // We hold the mutex guard
    Locked(OwnedMutexGuard<ContentWrapper<T>>),
}

#[derive(Default)]
pub struct ContentWrapper<T> {
    pub(crate) content: T,
    pub(crate) waker: Option<Waker>,
}

impl<T> ContentWrapper<T> {
    pub fn into_content(self) -> T {
        self.content
    }

    pub fn content(&self) -> &T {
        &self.content
    }

    pub(crate) fn take_content(&mut self) -> T
    where
        T: Default,
    {
        mem::take(&mut self.content)
    }
}

impl<T> LockState<T> {}
