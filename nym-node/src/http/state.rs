// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Clone)]
pub(crate) struct AppState {
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
        AppState {}
    }
}
