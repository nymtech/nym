// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

/// Shared application state available to all axum request handlers.
#[derive(Clone)]
pub(crate) struct AppState {
    //
}

impl AppState {
    pub(crate) fn new() -> Self {
        AppState {}
    }
}
