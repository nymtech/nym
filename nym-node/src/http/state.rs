// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use tokio::time::Instant;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    pub(crate) startup_time: Instant,
    // wireguard: WireguardAppState,
}

// #[derive(Debug, Clone)]
// pub struct WireguardAppState {
//     // inner: Option<WireguardAppStateInner>,
// }
//
// #[derive(Debug)]
// pub(crate) struct WireguardAppStateInner {
//     //
// }
//
// impl FromRef<AppState> for WireguardAppState {
//     fn from_ref(app_state: &AppState) -> Self {
//         app_state.wireguard.clone()
//     }
// }

impl AppState {
    pub fn new() -> Self {
        AppState {
            // is it 100% accurate?
            // no.
            // does it have to be?
            // also no.
            startup_time: Instant::now(),
        }
    }
}
