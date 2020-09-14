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

use crate::error::GatewayClientError;
use futures::{Stream, StreamExt};
pub use packet_router::{
    AcknowledgementReceiver, AcknowledgementSender, MixnetMessageReceiver, MixnetMessageSender,
}; // this should be refactored away. the only reason it's used like this is to not break import paths
use tungstenite::{protocol::Message, Error as WsError};

pub mod error;
pub mod packet_router;

// right now the client itself is not wasm-compatible...

#[cfg(not(target_arch = "wasm32"))]
pub mod client;
#[cfg(not(target_arch = "wasm32"))]
pub mod socket_state;

#[cfg(not(target_arch = "wasm32"))]
pub use client::GatewayClient;

/// A helper method to read an underlying message from the stream or return an error.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn read_ws_stream_message<S>(conn: &mut S) -> Result<Message, GatewayClientError>
where
    S: Stream<Item = Result<Message, WsError>> + Unpin,
{
    match conn.next().await {
        Some(msg) => match msg {
            Ok(msg) => Ok(msg),
            Err(err) => Err(GatewayClientError::NetworkError(err)),
        },
        None => Err(GatewayClientError::ConnectionAbruptlyClosed),
    }
}
