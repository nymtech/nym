// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    inner: Arc<AppStateInner>,
}

// TODO: https://docs.rs/axum/latest/axum/extract/struct.State.html#substates

impl AppState {
    pub fn new() -> Self {
        AppState {
            inner: Arc::new(AppStateInner {}),
        }
    }
}

// TODO: that's kinda abusing it...
impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
pub(crate) struct AppStateInner {}
