// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
