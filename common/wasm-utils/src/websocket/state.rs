// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use web_sys::WebSocket;

// convenience wrapper for state values provided by [`web_sys::WebSocket`]
/// The state values correspond to the `readyState` API of the `WebSocket`.
/// See [MDN](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/readyState) for more details.
#[repr(u16)]
pub(crate) enum State {
    Connecting = 0,
    Open = 1,
    Closing = 2,
    Closed = 3,
}

impl From<u16> for State {
    fn from(state: u16) -> Self {
        match state {
            WebSocket::CONNECTING => State::Connecting,
            WebSocket::OPEN => State::Open,
            WebSocket::CLOSING => State::Closing,
            WebSocket::CLOSED => State::Closed,
            n => panic!("{} is not a valid WebSocket state!", n), // should we panic here or change it into `TryFrom` instead?
        }
    }
}
